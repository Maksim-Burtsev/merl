import Warehouse

let weightLimit = 30

extension Tariff {
    func doubled() -> Int {
        rate() * 2
        // ^ d: picker Sources/Shop/Pricing.swift:13, Sources/Shop/Pricing.swift:29; want Sources/Shop/Pricing.swift:13 (#380)
    }
}

public struct Basket {
    let tariff: Tariff
    //          ^ d: picker Sources/Shop/Basket.swift:5, Sources/Shop/Pricing.swift:8; want Sources/Shop/Pricing.swift:8 (#371)
    let coupon: Coupon?
    //          ^ d: Sources/Shop/Pricing.swift:21
    var owner = ""

    func gross() -> Money {
        //          ^ d: Sources/Shop/Pricing.swift:51
        discount(tariff.rate())
        // ^ d: Sources/Shop/Pricing.swift:60
        //              ^ d: picker Sources/Shop/Pricing.swift:13, Sources/Shop/Pricing.swift:29; want Sources/Shop/Pricing.swift:13 (#384)
    }

    func bonus() -> Int {
        (coupon?.rate() ?? 0) + gross()
        //       ^ d: picker Sources/Shop/Pricing.swift:13, Sources/Shop/Pricing.swift:29; want Sources/Shop/Pricing.swift:29 (#384)
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
    //^ d: picker Sources/Shop/Pricing.swift:16, Sources/Shop/Pricing.swift:32; want Sources/Shop/Pricing.swift:16 (#384)
    //               ^ d: picker Sources/Shop/Pricing.swift:16, Sources/Shop/Pricing.swift:32; want Sources/Shop/Pricing.swift:32 (#384)
}

func restock(weigh: Int) -> Int {
    weigh + weightLimit
    // ^ d: Sources/Warehouse/Warehouse.swift:8; want Sources/Shop/Basket.swift:48 (#366)
    //         ^ d: picker Sources/Shop/Basket.swift:3, Sources/Shop/Basket.swift:55; want Sources/Shop/Basket.swift:3 (#371)
}

func overweight(_ grams: Int) -> Bool {
    let weightLimit = 50
    return weigh(grams) > weightLimit
    //     ^ d: Sources/Warehouse/Warehouse.swift:8
    //                    ^ d: picker Sources/Shop/Basket.swift:3, Sources/Shop/Basket.swift:55; want Sources/Shop/Basket.swift:55 (#366)
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
        //    ^ d: picker Sources/Shop/Basket.swift:160, Sources/Shop/Pricing.swift:44; want Sources/Shop/Pricing.swift:44 (#380)
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
    //           ^ d: none; want Sources/Shop/Pricing.swift:68 (#463)
}

func code(_ c: Coupon) -> String {
    c.code + "x".parcel
    //^ d: picker Sources/Shop/Basket.swift:104, Sources/Shop/Pricing.swift:22; want Sources/Shop/Pricing.swift:22 (#384)
    //           ^ d: Sources/Warehouse/Warehouse.swift:19
}

func entries(_ l: Ledger) async -> Int {
    await l.entries.count
    //      ^ d: picker Sources/Shop/Basket.swift:110, Sources/Shop/Pricing.swift:54; want Sources/Shop/Pricing.swift:54 (#384)
}

func handle(_ o: Offer?, items: [Int]) {
    if let o {
        print(o)
        //    ^ d: none; want Sources/Shop/Basket.swift:115 (#366)
    }
    for item in items {
        print(item)
        //    ^ d: none; want Sources/Shop/Basket.swift:120 (#366)
    }
    items.forEach { entry in
        print(entry)
        //    ^ d: none; want Sources/Shop/Basket.swift:124 (#366)
    }
    do {
        try check()
    } catch {
        print(error)
        //    ^ d: none
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
    //                                ^ d: none; want Sources/Shop/Basket.swift:147 (#375)
    items.first
}

let label: String = "shop"
//         ^ d: Sources/Warehouse/Warehouse.swift:18; want none (#371)

func sealed(_ p: Parcel) -> Parcel.Seal {
    //                             ^ d: Sources/Shop/Pricing.swift:82
    p.sealed()
}

func cut() -> Offer {
    .cut(1)
    //^ d: picker Sources/Shop/Basket.swift:160, Sources/Shop/Pricing.swift:44; want Sources/Shop/Pricing.swift:44 (#380)
}

func items() -> Int {
    let basket = Basket(tariff: Tariff(base: 1), coupon: nil)
    let basket2 = basket
    //            ^ d: Sources/Shop/Basket.swift:166
    return basket2.bonus()
}

func renamed(_ courier: Courier) -> Courier {
    let courier = Courier(name: courier.name)
    //                          ^ d: picker Sources/Shop/Basket.swift:62; want Sources/Shop/Basket.swift:172 (#317)
    return courier
}

struct RefreshWindow {
    let maximumAttempts: Int
    init(interval: Double = 30, maximumAttempts: Int = 5) {
        self.maximumAttempts = maximumAttempts
    }
}

func window() -> Int {
    let w = RefreshWindow.init(interval: 30, maximumAttempts: 1)
    //                                       ^ d: Sources/Shop/Basket.swift:180
    //                                       status: maximumAttempts: parameter of RefreshWindow
    print(w, separator: "")
    //       ^ d: none
    //       status: separator: argument label
    return w.maximumAttempts
}
