// A typed name rebound to another project type with the same method, for a plain call: a one-line
// block-local `let`, a `guard let …;` and a projected `$x` keep a picker (#1341).
final class Torch {
    func flare() -> Int { 1 }
}

final class Ember {
    func flare() -> Int { 2 }
}

struct Spark: Error {
    func flare() -> Int { 3 }
}

enum Grate {
    case held(Ember)
}

@propertyWrapper
struct Glow {
    var wrappedValue: Torch
    var projectedValue: Ember { Ember() }
}

final class Brazier {
    var torch: Torch = Torch()
    @Glow var glim: Torch = Torch()

    func own(torch: Ember) -> Int {
        torch.flare()
        //    ^ d: Sources/Shop/Rebound.swift:8
    }

    func plain() -> Int {
        torch.flare() + self.torch.flare()
        //    ^ d: Sources/Shop/Rebound.swift:4
    }

    func plainSelf() -> Int {
        self.torch.flare()
        //         ^ d: Sources/Shop/Rebound.swift:4
    }

    func projected() -> Int {
        $glim.flare()
        //    ^ d: picker Sources/Shop/Rebound.swift:4, Sources/Shop/Rebound.swift:8, Sources/Shop/Rebound.swift:12; want Sources/Shop/Rebound.swift:8 (#1341)
    }

    func wrapped() -> Int {
        glim.flare()
        //   ^ d: Sources/Shop/Rebound.swift:4
    }
}

func lights(torch: Torch, maybe: Ember?, embers: [Ember], grate: Grate) throws {
    torch.flare()
    //    ^ d: Sources/Shop/Rebound.swift:4
    if let torch = maybe { _ = torch.flare() }
    //                               ^ d: picker Sources/Shop/Rebound.swift:4, Sources/Shop/Rebound.swift:8, Sources/Shop/Rebound.swift:12; want Sources/Shop/Rebound.swift:8 (#1341)
    if let torch = maybe {
        _ = torch.flare()
        //        ^ d: Sources/Shop/Rebound.swift:8
    }
    while let torch = maybe { _ = torch.flare() }
    //                                  ^ d: picker Sources/Shop/Rebound.swift:4, Sources/Shop/Rebound.swift:8, Sources/Shop/Rebound.swift:12; want Sources/Shop/Rebound.swift:8 (#1341)
    while let torch = maybe {
        _ = torch.flare()
        //        ^ d: Sources/Shop/Rebound.swift:8
    }
    for torch in embers { _ = torch.flare() }
    //                              ^ d: picker Sources/Shop/Rebound.swift:4, Sources/Shop/Rebound.swift:8, Sources/Shop/Rebound.swift:12; want Sources/Shop/Rebound.swift:8 (#1341)
    for torch in embers {
        _ = torch.flare()
        //        ^ d: Sources/Shop/Rebound.swift:8
    }
    switch grate { case let .held(torch): _ = torch.flare() }
    //                                              ^ d: picker Sources/Shop/Rebound.swift:4, Sources/Shop/Rebound.swift:8, Sources/Shop/Rebound.swift:12; want Sources/Shop/Rebound.swift:8 (#1341)
    switch grate {
    case let .held(torch):
        _ = torch.flare()
        //        ^ d: picker Sources/Shop/Rebound.swift:4, Sources/Shop/Rebound.swift:8, Sources/Shop/Rebound.swift:12; want Sources/Shop/Rebound.swift:8 (#1341)
    }
    if case let .held(torch) = grate { _ = torch.flare() }
    //                                           ^ d: picker Sources/Shop/Rebound.swift:4, Sources/Shop/Rebound.swift:8, Sources/Shop/Rebound.swift:12; want Sources/Shop/Rebound.swift:8 (#1341)
    if case let .held(torch) = grate {
        _ = torch.flare()
        //        ^ d: picker Sources/Shop/Rebound.swift:4, Sources/Shop/Rebound.swift:8, Sources/Shop/Rebound.swift:12; want Sources/Shop/Rebound.swift:8 (#1341)
    }
    do { try kindle() } catch let torch as Spark { _ = torch.flare() }
    //                                                       ^ d: picker Sources/Shop/Rebound.swift:4, Sources/Shop/Rebound.swift:8, Sources/Shop/Rebound.swift:12; want Sources/Shop/Rebound.swift:12 (#1341)
    do {
        try kindle()
    } catch let torch as Spark {
        _ = torch.flare()
        //        ^ d: picker Sources/Shop/Rebound.swift:4, Sources/Shop/Rebound.swift:8, Sources/Shop/Rebound.swift:12; want Sources/Shop/Rebound.swift:12 (#1341)
    }
    _ = embers.map { torch in torch.flare() }
    //                              ^ d: picker Sources/Shop/Rebound.swift:4, Sources/Shop/Rebound.swift:8, Sources/Shop/Rebound.swift:12; want Sources/Shop/Rebound.swift:8 (#1341)
    _ = embers.map { torch in
        torch.flare()
        //    ^ d: picker Sources/Shop/Rebound.swift:4, Sources/Shop/Rebound.swift:8, Sources/Shop/Rebound.swift:12; want Sources/Shop/Rebound.swift:8 (#1341)
    }
    _ = embers.map { (torch: Ember) -> Int in
        let lead = 0
        return lead + torch.flare()
        //                  ^ d: Sources/Shop/Rebound.swift:8
    }
    _ = embers.map {
        $0.flare()
        // ^ d: picker Sources/Shop/Rebound.swift:4, Sources/Shop/Rebound.swift:8, Sources/Shop/Rebound.swift:12; want Sources/Shop/Rebound.swift:8 (#1341)
    }
    func inner(torch: Ember) -> Int { torch.flare() }
    //                                      ^ d: picker Sources/Shop/Rebound.swift:4, Sources/Shop/Rebound.swift:8, Sources/Shop/Rebound.swift:12; want Sources/Shop/Rebound.swift:8 (#1341)
    func inner2(torch: Ember) -> Int {
        torch.flare()
        //    ^ d: Sources/Shop/Rebound.swift:8
    }
    do { let torch = Ember(); _ = torch.flare() }
    //                                  ^ d: picker Sources/Shop/Rebound.swift:4, Sources/Shop/Rebound.swift:8, Sources/Shop/Rebound.swift:12; want Sources/Shop/Rebound.swift:8 (#1341)
    do {
        let torch = Ember()
        _ = torch.flare()
        //        ^ d: Sources/Shop/Rebound.swift:8
    }
    torch.flare()
    //    ^ d: Sources/Shop/Rebound.swift:4
    if let torch = maybe { _ = torch }; _ = torch.flare()
    //                                            ^ d: Sources/Shop/Rebound.swift:4
    _ = torch.flare(); if let torch = maybe { _ = torch }
    //        ^ d: Sources/Shop/Rebound.swift:4
    _ = torch.flare() // if let torch = maybe {
    //        ^ d: Sources/Shop/Rebound.swift:4
    print("for torch in embers {", torch.flare())
    //                                   ^ d: Sources/Shop/Rebound.swift:4
    do { let torch = Ember(); _ = torch }; _ = torch.flare()
    //                                               ^ d: Sources/Shop/Rebound.swift:4
    var torch2 = torch; let lead = 0; _ = torch.flare() + lead
    //                                          ^ d: Sources/Shop/Rebound.swift:4
    _ = "let torch = x;"; _ = torch.flare(); torch2 = torch
    //                              ^ d: Sources/Shop/Rebound.swift:4
    guard let torch = maybe else { return }
    _ = torch.flare()
    //        ^ d: Sources/Shop/Rebound.swift:8
}

func locals(maybe: Ember?) {
    let torch = Torch()
    if let torch = maybe { _ = torch.flare() }
    //                               ^ d: picker Sources/Shop/Rebound.swift:4, Sources/Shop/Rebound.swift:8, Sources/Shop/Rebound.swift:12; want Sources/Shop/Rebound.swift:8 (#1341)
    guard let torch = maybe else { return }; _ = torch.flare()
    //                                                 ^ d: picker Sources/Shop/Rebound.swift:4, Sources/Shop/Rebound.swift:8, Sources/Shop/Rebound.swift:12; want Sources/Shop/Rebound.swift:8 (#1341)
}

func kindle() throws {}
