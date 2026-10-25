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
