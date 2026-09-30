import Foundation

// A function's parameters and what `if let`, `for`, `catch`, a closure and `guard let` bind
// (#366), each with a namesake elsewhere that `d` found by name.
func route(_ review: Review, tries: Int, queue: DispatchQueue) {
    if let fault = review.fault {
        fail(fault)
        //   ^ d: Sources/Shop/Dispatcher.swift:6
        //     status: (local)
    }
    for attempt in 0..<tries {
        log(attempt)
        //  ^ d: Sources/Shop/Dispatcher.swift:11
    }
    do {
        try run()
    } catch {
        report(error)
        //     ^ d: Sources/Shop/Dispatcher.swift:17
    }
    queue.async { crate in
        use(crate)
        //  ^ d: Sources/Shop/Dispatcher.swift:21
    }
    guard let job = review.job else { return }
    log(job, tries)
    //  ^ d: Sources/Shop/Dispatcher.swift:25
    //       ^ d: Sources/Shop/Dispatcher.swift:5
}

func pick(_ offer: Offer, fallback: Int) -> Int {
    switch offer {
    case let .cut(fallback):
        return fallback
        //     ^ d: Sources/Shop/Dispatcher.swift:33
    case .bundle(let count):
        return count + fallback
        //     ^ d: Sources/Shop/Dispatcher.swift:36
        //             ^ d: Sources/Shop/Dispatcher.swift:31
    default:
        return fallback
        //     ^ d: Sources/Shop/Dispatcher.swift:31
    }
}

func wrapped(
    first: Int,
    then second: Int
) -> Int {
    if let doubled = Optional(first * 2),
       let tripled = Optional(second * 3) {
        return doubled + tripled
        //     ^ d: Sources/Shop/Dispatcher.swift:50
        //               ^ d: Sources/Shop/Dispatcher.swift:51
    }
    for (index, value) in [first].enumerated() {
        return index + value
        //             ^ d: Sources/Shop/Dispatcher.swift:56
    }
    return second
    //     ^ d: Sources/Shop/Dispatcher.swift:48
}

// What the walk must not read: a pattern it cannot read hides nothing it would find, a closure
// closed above the cursor binds nothing there, and the top of a file binds no local.
func unpack(_ pair: (Int, (Int, Int)), inner: Int) -> Int {
    let (outer, (inner, rest)) = pair
    return inner + outer + rest
    //     ^ d: none
}

func sweep(_ slot: Int, crates: [Int]) -> Int {
    crates.forEach { slot in
        print(slot)
    }
    return slot
    //     ^ d: Sources/Shop/Dispatcher.swift:72
}

let shelf = 1
let shelfTwice = shelf * 2
//               ^ d: Sources/Shop/Dispatcher.swift:80
//                 status: by name

func aligned(lead: Int,
             trail: Int) -> Int {
    lead + trail
    //     ^ d: Sources/Shop/Dispatcher.swift:86
}

// On the name a binding declares, on its own line, `d` answers as on a `let` (#525): at a
// declaration, the namesakes `Tally` declares offered, never jumped to, and the line itself
// when it has none. A use on the line is looked up as anywhere.
func retry(_ review: Review, tries: Int, queue: DispatchQueue) {
    if let fault = review.fault {
    //     ^ d: picker Sources/Shop/Tally.swift:2
    //       status: fault: at a declaration, 1 other by name
    //             ^ d: Sources/Shop/Dispatcher.swift:94
        fail(fault)
    }
    for attempt in 0..<tries {
    //  ^ d: picker Sources/Shop/Tally.swift:7
    //    status: at a declaration
    //                 ^ d: Sources/Shop/Dispatcher.swift:94
        log(attempt)
    }
    queue.async { crate in
    //            ^ d: picker Sources/Shop/Tally.swift:8
    //              status: at a declaration
        use(crate)
    }
    guard let job = review.job else { return }
    //        ^ d: picker Sources/Shop/Tally.swift:3
    //          status: at a declaration
    for parcelIndex in 0..<tries {
    //  ^ d: Sources/Shop/Dispatcher.swift:115
        log(parcelIndex)
    }
}

// The name a header binds is the occurrence it binds, not the first on its line (#525): a read
// in front of the binding is looked up as on any other line.
struct Shelf {
    var stock: [Int] = []
    var spare: Int?
    var pallets: [Int] = []

    func refresh() {
        if stock.isEmpty, let stock = load() {
        // ^ d: Sources/Shop/Dispatcher.swift:124
        //                    ^ d: picker Sources/Shop/Dispatcher.swift:124
        //                      status: at a declaration
            use(stock)
        }
        pallets.forEach { pallets in
        // ^ d: Sources/Shop/Dispatcher.swift:126
        //                ^ d: picker Sources/Shop/Dispatcher.swift:126
        //                  status: at a declaration
            use(pallets)
        }
        guard spare != nil, let spare = spare else { return }
        //    ^ d: Sources/Shop/Dispatcher.swift:125
        //                      ^ d: picker Sources/Shop/Dispatcher.swift:125
        //                        status: at a declaration
        //                              ^ d: Sources/Shop/Dispatcher.swift:125
    }
}
