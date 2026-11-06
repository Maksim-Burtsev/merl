// A label inside an implicit `.init(` passed to an outer call is typed by the outer callee's
// parameter of that label (#1341). Overloads, generics, aliases and namesakes keep it unanswered.
final class Lease<Grantor> {
    struct Span {
        init(interval: Double = 30, tries: Int = 5) {}
    }

    init(grantor: Grantor, holder: String = "", span: Span? = Span()) {}

    func renew(span: Span) {}
}

final class Valve {
    struct Gauge {
        init(tries: Int) {}
    }

    struct Dial {
        init(tries: Int) {}
    }

    init(gauge: Gauge) {}
    init(gauge: Dial, strict: Bool = false) {}
}

final class Sluice<Pane> {
    init(pane: Pane) {}
    init<Slat>(slat: Slat) {}
}

enum Weir {
    struct Shutter {
        init(tries: Int) {}
    }
}

enum Dam {
    struct Shutter {
        init(tries: Int) {}
    }

    typealias Flap = Lease<Int>.Span

    init(shutter: Shutter) {}
    init(flap: Flap) {}
    init(_ span: Lease<Int>.Span) {}
}

final class Spool {
    struct Coil {
        init(tries: Int) {}
    }

    func renew(span: Coil) {}
}

func leases(flag: Bool, lease: Lease<Int>) {
    _ = Lease(grantor: 1,
              span: .init(interval: 30, tries: 1))
    //                                    ^ d: Sources/Shop/Refresh.swift:5
    //                                    status: tries: parameter of Lease.Span
    _ = Lease(grantor: 1, holder: "a", span: .init(tries: 2))
    //                                             ^ d: Sources/Shop/Refresh.swift:5
    _ = Lease(grantor: 1, span: flag ? .init(tries: 1) : nil)
    //                                       ^ d: !jump
    _ = Valve(gauge: .init(tries: 1))
    //                     ^ d: !jump
    _ = Sluice(pane: .init(tries: 1))
    //                     ^ d: !jump
    _ = Sluice<Int>(slat: .init(tries: 1))
    //                          ^ d: !jump
    _ = Dam(shutter: .init(tries: 1))
    //                     ^ d: !jump
    _ = Dam(flap: .init(tries: 1))
    //                  ^ d: !jump
    _ = Dam(.init(tries: 1))
    //            ^ d: !jump
    _ = [Lease(grantor: 1, span: [.init(tries: 1)].first)]
    //                                  ^ d: !jump
    _ = { (lease: Spool) in lease.renew(span: .init(tries: 1)) }
    //                                                   ^ d: !jump
    _ = lease.renew(span: .init(tries: 1))
    //                              ^ d: none; want Sources/Shop/Refresh.swift:5 (#1341)
}
