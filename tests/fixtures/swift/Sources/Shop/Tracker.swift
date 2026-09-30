// `d` on a function's locals and on a type's extensions (#371).
public final class Tracker {
    let span: Double = 10

    func poll(_ path: String) -> Int {
        0
    }

    func retry() {
        let span = 5
        print(span)
    }

    func resume() {
        print(span)
        //    ^ d: Sources/Shop/Tracker.swift:3
        //      status: span → Tracker.span (via self: Tracker)
    }
}

extension Tracker {
    var idle: Bool { true }
}

public protocol Tracked {
    var stamp: Int { get }
}

func stamped(_ t: Tracked) -> Int {
    let stamp = 2
    return t.stamp + stamp
    //       ^ d: Sources/Shop/Tracker.swift:26
}

let lastTracker: Tracker? = nil
//               ^ d: Sources/Shop/Tracker.swift:2
//                 status: Tracker: by name, 1 match

func restock() -> Int {
    let quota = 3
    func refill() -> Int {
        quota
        // ^ d: Sources/Shop/Tracker.swift:40
    }
    return refill()
}

// A nested function reads its outer function's locals, which the walk over the blocks does not
// see: no proof of no local, so no jump to the type's namesake member (#380).
extension Tracker {
    func cruise() -> Double {
        let span = 5.0
        func drift() -> Double {
            span
            // ^ d: picker Sources/Shop/Tracker.swift:3, Sources/Shop/Tracker.swift:52
        }
        return drift()
    }
}
