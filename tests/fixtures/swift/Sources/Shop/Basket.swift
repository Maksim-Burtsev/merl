import Warehouse

let weightLimit = 30

extension Tariff {
    func doubled() -> Int {
        rate() * 2
        // ^ d: Sources/Shop/Pricing.swift:13
    }
}

public struct Basket {
    let tariff: Tariff
    //          ^ d: Sources/Shop/Pricing.swift:8
    let coupon: Coupon?
    //          ^ d: Sources/Shop/Pricing.swift:21
    var owner = ""

    func gross() -> Money {
        //          ^ d: Sources/Shop/Pricing.swift:51
        discount(tariff.rate())
        // ^ d: Sources/Shop/Pricing.swift:60
        //              ^ d: Sources/Shop/Pricing.swift:13
    }

    func bonus() -> Int {
        (coupon?.rate() ?? 0) + gross()
        //       ^ d: Sources/Shop/Pricing.swift:29
        //                      ^ d: Sources/Shop/Basket.swift:19
    }
}

extension Basket: Priced {
    //            ^ d: Sources/Shop/Pricing.swift:37
    typealias Amount = Int
    func price() -> Int {
        bonus()
        // ^ d: Sources/Shop/Basket.swift:26
    }
}

func describeAny(_ t: Tariff, _ c: Coupon) -> String {
    t.describe() + c.describe()
    //^ d: Sources/Shop/Pricing.swift:16
    //               ^ d: Sources/Shop/Pricing.swift:32
}

func restock(weigh: Int) -> Int {
    weigh + weightLimit
    // ^ d: Sources/Shop/Basket.swift:48
    //         ^ d: Sources/Shop/Basket.swift:3
}

func overweight(_ grams: Int) -> Bool {
    let weightLimit = 50
    return weigh(grams) > weightLimit
    //     ^ d: Sources/Warehouse/Warehouse.swift:8
    //                    ^ d: Sources/Shop/Basket.swift:55
}

func dispatch() -> String {
    let courier = Courier(name: "post")
    //            ^ d: Sources/Warehouse/Warehouse.swift:1
    //                    ^ d: Sources/Warehouse/Warehouse.swift:3
    return courier.name
    //             ^ d: Sources/Warehouse/Warehouse.swift:2
}

func offer(_ o: Offer) -> Int {
    switch o {
    case .plain:
        //^ d: Sources/Shop/Pricing.swift:43
        return 0
    case let .cut(n):
        //    ^ d: Sources/Shop/Pricing.swift:44
        return n
    case .bundle(let count):
        // ^ d: Sources/Shop/Pricing.swift:44
        return count
    }
}

func depot() -> Bool {
    Depot.open()
    // ^ d: Sources/Warehouse/Warehouse.swift:12
    //    ^ d: Sources/Warehouse/Warehouse.swift:13
}

func money(_ ledger: Ledger) async -> Money {
    //               ^ d: Sources/Shop/Pricing.swift:53
    let level = Level.low
    //                ^ d: Sources/Shop/Pricing.swift:48
    return await ledger[level.rawValue] + rateCap + currency.count
    //                                    ^ d: Sources/Shop/Pricing.swift:5
    //                                              ^ d: Sources/Shop/Pricing.swift:6
}

func pay(_ b: Basket) -> Int {
    settle(b) + `default`()
    // ^ d: Sources/Shop/Pricing.swift:64
    //           ^ d: Sources/Shop/Pricing.swift:68
}

func code(_ c: Coupon) -> String {
    c.code + "x".parcel
    //^ d: Sources/Shop/Pricing.swift:22
    //           ^ d: Sources/Warehouse/Warehouse.swift:19
}

func entries(_ l: Ledger) async -> Int {
    await l.entries.count
    //      ^ d: Sources/Shop/Pricing.swift:54
}

func handle(_ o: Offer?, items: [Int]) {
    if let o {
        print(o)
        //    ^ d: Sources/Shop/Basket.swift:115
    }
    for item in items {
        print(item)
        //    ^ d: Sources/Shop/Basket.swift:120
    }
    items.forEach { entry in
        print(entry)
        //    ^ d: Sources/Shop/Basket.swift:124
    }
    do {
        try check()
    } catch {
        print(error)
        //    ^ d: Sources/Shop/Basket.swift:130
    }
}

func check() throws {}

func weighed(_ c: Courier) -> String {
    Courier(name: c.name).name
    //      ^ d: Sources/Warehouse/Warehouse.swift:3
}

let amount: Priced.Type? = nil
//          ^ d: Sources/Shop/Pricing.swift:37
//                 ^ d: none

func cheapest<Item: Priced>(_ items: [Item]) -> Item? {
    //                                ^ d: Sources/Shop/Basket.swift:147
    items.first
}

let label: String = "shop"
//         ^ d: picker Sources/Warehouse/Warehouse.swift:18

func sealed(_ p: Parcel) -> Parcel.Seal {
    //                             ^ d: Sources/Shop/Pricing.swift:82
    p.sealed()
}

func cut() -> Offer {
    .cut(1)
    //^ d: Sources/Shop/Pricing.swift:44
}

func items() -> Int {
    let basket = Basket(tariff: Tariff(base: 1), coupon: nil)
    let basket2 = basket
    //            ^ d: Sources/Shop/Basket.swift:166
    return basket2.bonus()
}

func renamed(_ courier: Courier) -> Courier {
    let courier = Courier(name: courier.name)
    //                          ^ d: Sources/Shop/Basket.swift:172
    return courier
}

func mode() -> Int {
    let m = Mode.open
    //           ^ d: Sources/Shop/Pricing.swift:92
    let f: Mode = .fast(1)
    //             ^ d: Sources/Shop/Pricing.swift:92
    return `tally`(1)
    //      ^ d: Sources/Shop/Pricing.swift:87
}

struct RefreshWindow {
    let maximumAttempts: Int
    init(interval: Double = 30, maximumAttempts: Int = 5) {
        self.maximumAttempts = maximumAttempts
    }
}

func window() -> Int {
    let w = RefreshWindow.init(interval: 30, maximumAttempts: 1)
    //                                       ^ d: Sources/Shop/Basket.swift:189
    //                                       status: maximumAttempts: parameter of RefreshWindow
    print(w, separator: "")
    //       ^ d: none
    //       status: separator: argument label
    return w.maximumAttempts
}

struct Bell {
    func chime() -> Int { 1 }
}

func ringBell(_ bell: Bell) -> Int {
    let chime = bell.chime()
    //               ^ d: Sources/Shop/Basket.swift:205
    return chime
}
