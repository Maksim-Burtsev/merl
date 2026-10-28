// An implicit `.x` passed as an argument reads the type of the callee's parameter, from the
// callee's own scope outward (#1286). Any doubt keeps the picker of every namesake.
struct Packer {
    enum Layout {
        case indexed, loose
    }

    init(layout: Layout = .loose, strict: Bool = false) {}

    static func pack(into layout: Layout, policy: any Policy) -> Packer { Packer(layout: layout) }

    static func spread<Layout>(_ layout: Layout) {}
}

struct Coder {
    enum Layout {
        case indexed, loose
    }

    init(layout: Layout) {}
}

enum Tray {
    case indexed
}

enum Bin {
    case indexed
}

struct Mover {
    static func move(_ tray: Tray) {}
    static func move(_ bin: Bin) {}
}

protocol Policy {}

struct Strict: Policy {}

extension Policy where Self == Strict {
    static var strict: Strict { Strict() }
}

func arguments() {
    _ = Packer(layout: .indexed)
    //                  ^ d: Sources/Shop/Arguments.swift:5
    _ = Coder(layout: .indexed)
    //                 ^ d: Sources/Shop/Arguments.swift:17
    _ = Packer.pack(into: .indexed, policy: .strict)
    //                     ^ d: Sources/Shop/Arguments.swift:5
    _ = Packer.pack(into: .loose, policy: .strict)
    //                                     ^ d: Sources/Shop/Arguments.swift:41
    Packer.spread(.indexed)
    //             ^ d: !jump
    Mover.move(.indexed)
    //          ^ d: picker Sources/Shop/Arguments.swift:24, Sources/Shop/Arguments.swift:28
    _ = Packer(layout: true ? .indexed : .loose)
    //                         ^ d: !jump
}

struct Porter {
    static func send(_ disposition: URLSession.ResponseDisposition) {}
}

func outside() {
    Porter.send(.allow)
    //           ^ d: none
}

// A memberwise init's property typed by its value, a case of a nested type, and a method of a
// value whose type merl knows read the parameter's type the same way (#1286).
enum Lane {
    case express, standard
}

struct Ticket {
    enum Lane {
        case express, standard
    }

    var lane = Lane.standard
    var rush = Rush.make()
}

enum Rush {
    case express

    static func make() -> Lane { .express }
}

final class Cashier {
    func file(_ lane: Lane, rush: Bool = false) {}
    func stamp(_ lane: Lane) {}
    func stamp(_ lane: Ticket.Lane) {}
}

func tickets(cashier: Cashier) {
    _ = Ticket(lane: .express)
    //                ^ d: Sources/Shop/Arguments.swift:78
    _ = Ticket(rush: .express)
    //                ^ d: !jump
    let desk = Cashier()
    desk.file(.express, rush: true)
    //         ^ d: Sources/Shop/Arguments.swift:73
    cashier.file(.express)
    //            ^ d: Sources/Shop/Arguments.swift:73
    cashier.stamp(.express)
    //             ^ d: !jump
    stranger.file(.express)
    //             ^ d: !jump
}

extension Lane {
    static var fallback: Lane { .standard }
}
