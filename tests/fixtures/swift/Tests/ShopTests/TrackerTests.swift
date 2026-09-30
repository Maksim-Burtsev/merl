@testable import Shop

final class TrackerTests {
    let tracker = Tracker()

    func testA() {
        let poll = tracker.poll("a")
        print(poll)
    }

    func testB() {
        _ = tracker.poll("b")
        //          ^ d: Sources/Shop/Tracker.swift:5
        //            status: poll → Tracker.poll (by name, 1 match)
    }
}
