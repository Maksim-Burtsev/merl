// An enum payload bound by a pattern, where the payload's type and the enum share a member (#1341).
enum Fault: Error {
    enum Snag {
        case frayed
        case torn(Int)
    }

    case snagged(reason: Snag)
    case pair(Snag, Snag)
    case bare(Snag)
    case nested(inner: Fault)

    var isFrayed: Bool {
        if case let .snagged(reason) = self, reason.isFrayed { return true }
        //                                          ^ d: Sources/Shop/Payloads.swift:45
        return false
    }

    var isFrayedBare: Bool {
        if case .bare(let reason) = self, reason.isFrayed { return true }
        //                                       ^ d: Sources/Shop/Payloads.swift:45
        return false
    }

    var isFrayedGuard: Bool {
        guard case let .snagged(reason) = self else { return false }
        return reason.isFrayed
        //            ^ d: Sources/Shop/Payloads.swift:45
    }

    var isFrayedPair: Bool {
        if case let .pair(a, b) = self, b.isFrayed { return a.isFrayed }
        //                                ^ d: Sources/Shop/Payloads.swift:45
        return false
    }

    var isFrayedNested: Bool {
        if case let .nested(reason) = self, reason.isFrayed { return true }
        //                                         ^ d: Sources/Shop/Payloads.swift:13
        return false
    }
}

extension Fault.Snag {
    var isFrayed: Bool {
        if case .frayed = self { return true }
        return false
    }
}

enum Rope {
    case snagged(reason: Fault)
    case pair(Fault.Snag, Fault.Snag)
}

enum Held<T> {
    case snagged(reason: T)
}

struct Spool {
    var isFrayed: Bool { false }
}

func payloads(rope: Rope, fault: Fault, held: Held<Fault.Snag>, reason: Spool) -> Bool {
    if case let .snagged(reason) = rope, reason.isFrayed { return true }
    //                                          ^ d: Sources/Shop/Payloads.swift:13
    if case let .snagged(reason) = held, reason.isFrayed { return true }
    //                                          ^ d: picker Sources/Shop/Payloads.swift:13, Sources/Shop/Payloads.swift:45, Sources/Shop/Payloads.swift:61
    if case let .pair(reason) = rope, reason.0.isFrayed { return true }
    //                                         ^ d: picker Sources/Shop/Payloads.swift:13, Sources/Shop/Payloads.swift:45, Sources/Shop/Payloads.swift:61
    if case let .snagged(reason) = fault, let reason = Spool() as Spool?, reason.isFrayed { return true }
    //                                                                           ^ d: picker Sources/Shop/Payloads.swift:13, Sources/Shop/Payloads.swift:45, Sources/Shop/Payloads.swift:61
    if case let .snagged(reason) = fault, case let .snagged(reason) = rope, reason.isFrayed { return true }
    //                                                                             ^ d: picker Sources/Shop/Payloads.swift:13, Sources/Shop/Payloads.swift:45, Sources/Shop/Payloads.swift:61
    if case .snagged(reason: let reason) = fault, reason.isFrayed { return true }
    //                                                   ^ d: Sources/Shop/Payloads.swift:45
    if case let .snagged(other) = fault, reason.isFrayed { return other.isFrayed }
    //                                          ^ d: picker Sources/Shop/Payloads.swift:13, Sources/Shop/Payloads.swift:45, Sources/Shop/Payloads.swift:61; want Sources/Shop/Payloads.swift:61 (#1341)
    if case let .snagged(reason) = fault {
        let reason = Spool()
        return reason.isFrayed
        //            ^ d: picker Sources/Shop/Payloads.swift:13, Sources/Shop/Payloads.swift:45, Sources/Shop/Payloads.swift:61; want Sources/Shop/Payloads.swift:61 (#1341)
    }
    if case let .snagged(reason) = fault { _ = { (reason: Spool) in reason.isFrayed } }
    //                                                                     ^ d: picker Sources/Shop/Payloads.swift:13, Sources/Shop/Payloads.swift:45, Sources/Shop/Payloads.swift:61; want Sources/Shop/Payloads.swift:61 (#1341)
    if case let .snagged(reason) = fault {
        return [Spool()].map { reason in reason.isFrayed }[0]
        //                                      ^ d: picker Sources/Shop/Payloads.swift:13, Sources/Shop/Payloads.swift:45, Sources/Shop/Payloads.swift:61
    }
    if case let .pair(a, b) = rope, a.isFrayed { return b.isFrayed }
    //                                ^ d: Sources/Shop/Payloads.swift:45
    return false
}

func shadowed(reason: Fault, flag: Bool) -> Bool {
    if case let .nested(reason) = reason, reason.isFrayed { return true }
    //                                           ^ d: picker Sources/Shop/Payloads.swift:13, Sources/Shop/Payloads.swift:45, Sources/Shop/Payloads.swift:61; want Sources/Shop/Payloads.swift:13 (#1341)
    if flag {
    } else if case let .snagged(reason) = reason, reason.isFrayed { return true }
    //                                                   ^ d: picker Sources/Shop/Payloads.swift:13, Sources/Shop/Payloads.swift:45, Sources/Shop/Payloads.swift:61; want Sources/Shop/Payloads.swift:45 (#1341)
    if case let .snagged(snag) = reason, case .frayed = snag { return snag.isFrayed }
    //                                                                     ^ d: picker Sources/Shop/Payloads.swift:13, Sources/Shop/Payloads.swift:45, Sources/Shop/Payloads.swift:61; want Sources/Shop/Payloads.swift:45 (#1341)
    return false
}

func arms(rope: Rope, fault: Fault, held: Held<Fault.Snag>, reason: Spool, flag: Bool) -> Bool {
    switch rope {
    case let .snagged(reason):
        return reason.isFrayed
        //            ^ d: Sources/Shop/Payloads.swift:13
    case .pair(let a, _):
        return a.isFrayed
        //       ^ d: Sources/Shop/Payloads.swift:45
    }
    switch fault {
        case .snagged(let reason):
            if flag { return reason.isFrayed }
            //                      ^ d: Sources/Shop/Payloads.swift:45
        case .bare:
            return reason.isFrayed
            //            ^ d: picker Sources/Shop/Payloads.swift:13, Sources/Shop/Payloads.swift:45, Sources/Shop/Payloads.swift:61; want Sources/Shop/Payloads.swift:61 (#1341)
        default:
            return false
    }
    switch fault { case let .snagged(reason): return reason.isFrayed }
    //                                                      ^ d: Sources/Shop/Payloads.swift:45
    switch fault { case let .snagged(reason): return reason.isFrayed; default: return reason.isFrayed }
    //                                                                                       ^ d: picker Sources/Shop/Payloads.swift:13, Sources/Shop/Payloads.swift:45, Sources/Shop/Payloads.swift:61
    switch held { case let .snagged(reason): return reason.isFrayed }
    //                                                     ^ d: picker Sources/Shop/Payloads.swift:13, Sources/Shop/Payloads.swift:45, Sources/Shop/Payloads.swift:61; want Sources/Shop/Payloads.swift:45 (#1341)
    switch fault {
    case let .snagged(reason) where reason.isFrayed:
        //                                 ^ d: picker Sources/Shop/Payloads.swift:13, Sources/Shop/Payloads.swift:45, Sources/Shop/Payloads.swift:61; want Sources/Shop/Payloads.swift:45 (#1341)
        return reason.isFrayed
        //            ^ d: picker Sources/Shop/Payloads.swift:13, Sources/Shop/Payloads.swift:45, Sources/Shop/Payloads.swift:61; want Sources/Shop/Payloads.swift:45 (#1341)
    case let .snagged(reason), let .bare(reason):
        return reason.isFrayed
        //            ^ d: picker Sources/Shop/Payloads.swift:13, Sources/Shop/Payloads.swift:45, Sources/Shop/Payloads.swift:61; want Sources/Shop/Payloads.swift:45 (#1341)
    case let .nested(reason):
        let reason = Spool()
        return reason.isFrayed
        //            ^ d: picker Sources/Shop/Payloads.swift:13, Sources/Shop/Payloads.swift:45, Sources/Shop/Payloads.swift:61; want Sources/Shop/Payloads.swift:61 (#1341)
    case .pair:
        switch rope {
        case let .snagged(reason):
            return reason.isFrayed
            //            ^ d: Sources/Shop/Payloads.swift:13
        default:
            return reason.isFrayed
            //            ^ d: picker Sources/Shop/Payloads.swift:13, Sources/Shop/Payloads.swift:45, Sources/Shop/Payloads.swift:61; want Sources/Shop/Payloads.swift:61 (#1341)
        }
    }
    return false
}

func shadowedArms(reason: Fault) -> Bool {
    switch reason {
    case let .nested(reason):
        return reason.isFrayed
        //            ^ d: picker Sources/Shop/Payloads.swift:13, Sources/Shop/Payloads.swift:45, Sources/Shop/Payloads.swift:61; want Sources/Shop/Payloads.swift:13 (#1341)
    default:
        return false
    }
    switch reason { case let .snagged(reason): return reason.isFrayed }
    //                                                       ^ d: picker Sources/Shop/Payloads.swift:13, Sources/Shop/Payloads.swift:45, Sources/Shop/Payloads.swift:61; want Sources/Shop/Payloads.swift:45 (#1341)
}

extension Fault {
    var isFrayedArm: Bool {
        switch self {
        case let .snagged(reason): reason.isFrayed
        //                                ^ d: Sources/Shop/Payloads.swift:45
        case .bare(let reason): reason.isFrayed
        //                             ^ d: Sources/Shop/Payloads.swift:45
        default: false
        }
    }
}

struct Reel {
    let reason = Spool()

    func spin(fault: Fault) -> Bool {
        switch fault {
        case let .snagged(reason):
            _ = reason
        case .bare:
            return reason.isFrayed
            //            ^ d: picker Sources/Shop/Payloads.swift:13, Sources/Shop/Payloads.swift:45, Sources/Shop/Payloads.swift:61; want Sources/Shop/Payloads.swift:61 (#1341)
        default:
            break
        }
        switch fault {
        case let .snagged(reason):
            _ = reason
        default:
            return reason.isFrayed
            //            ^ d: picker Sources/Shop/Payloads.swift:13, Sources/Shop/Payloads.swift:45, Sources/Shop/Payloads.swift:61; want Sources/Shop/Payloads.swift:61 (#1341)
        }
        return reason.isFrayed
        //            ^ d: picker Sources/Shop/Payloads.swift:13, Sources/Shop/Payloads.swift:45, Sources/Shop/Payloads.swift:61; want Sources/Shop/Payloads.swift:61 (#1341)
    }
}
