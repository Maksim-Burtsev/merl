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
        //      status: span → Tracker.span (by name, 1 match)
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
