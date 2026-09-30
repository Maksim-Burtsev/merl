// How a member's kind narrows `d` (#380): an implicit member `.word` is an enum case or a
// `static` member, `Type.word` on a type the project declares is one of its cases or `static`
// members, and a bare word in a type's body is that type's member first.
struct Endpoint {
    enum Path {
        case bytes(count: Int)
    }

    static func method(_ name: String) -> Endpoint {
        Endpoint(path: .bytes(count: 1), method: name)
        //              ^ d: Sources/Shop/Endpoint.swift:6
    }

    let path: Path
    var method: String = "GET"
}

struct Stats {
    let bytes: Int
}

func summarize(_ endpoint: Endpoint, stats: Stats) {
    switch endpoint.path {
    case let .bytes(count):
    //        ^ d: Sources/Shop/Endpoint.swift:6
    //          status: bytes → Endpoint.Path.bytes
        print(count)
    }
    _ = Endpoint.method("POST")
    //           ^ d: Sources/Shop/Endpoint.swift:9
    //             status: method → Endpoint.method (via Endpoint)
    // A value's member: its type's, and an instance member over a `static` one (#384).
    _ = endpoint.method
    //           ^ d: Sources/Shop/Endpoint.swift:15
    //             status: via endpoint: Endpoint
    _ = stats.bytes
    //        ^ d: Sources/Shop/Endpoint.swift:19
}
