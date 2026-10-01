protocol Lock {
    func lock()
}

extension Lock {
    func around() {
        lock()
    //  ^ d: Sources/Shop/Protected.swift:2
    //    status: lock → Lock.lock (via self: Lock)
    }

    func hold(lock: Int) {
        print(lock)
        //    ^ d: Sources/Shop/Protected.swift:12
    }
}

final class UnfairLock {
    private let lock = 0
}

class Guarded {}

// Constrained to another type: the member may be `Guarded`'s, and the search by name decides.
extension Lock where Self: Guarded {
    func twice() {
        lock()
    //  ^ d: picker Sources/Shop/Protected.swift:2, Sources/Shop/Protected.swift:19
    }
}
