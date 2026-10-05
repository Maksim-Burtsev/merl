module Shop.Courier

open Shop.Orders
//        ^ d: src/Pricing.fs:1

let limit = Ledger.Limit + Ledger.fee + Ledger.twice 2 + Ledger.counter
//                 ^ d: src/Pricing.fs:5
//                 status: via Ledger
//                                ^ d: src/Pricing.fs:6
//                                             ^ d: src/Pricing.fs:7
//                                                              ^ d: src/Pricing.fs:8

let sum = Ledger.add 1 2 + Shop.Orders.Ledger.loop 0 []
//        ^ d: src/Pricing.fs:3
//               ^ d: src/Pricing.fs:9
//                                            ^ d: src/Pricing.fs:10

let area (s: Shape) =
//           ^ d: src/Pricing.fs:17
    match s with
    | Circle r -> r * r
//    ^ d: src/Pricing.fs:18
    | Square a -> a * a
//    ^ d: src/Pricing.fs:19

let closed = Closed
//           ^ d: src/Pricing.fs:21

let account = Account.Create(1)
//            ^ d: src/Pricing.fs:23
//                    ^ d: src/Pricing.fs:29

let shown (a: Account) = a.Total + a.Id + a.seen + a.ToString().Length
//                         ^ d: src/Pricing.fs:26
//                                   ^ d: src/Pricing.fs:27
//                                          ^ d: src/Pricing.fs:25
//                                                   ^ d: src/Pricing.fs:28

let area2 (f: Figure) = f.Area
//            ^ d: src/Pricing.fs:32
//                        ^ d: picker src/Pricing.fs:33, src/Pricing.fs:34

let find (r: IRepo) = r.Find 1
//           ^ d: src/Pricing.fs:36
//                      ^ d: src/Pricing.fs:37

let rate (c: Coupon) = c.Rate + c.Code.Length
//                       ^ d: src/Pricing.fs:39
//                                ^ d: src/Pricing.fs:39

let missing () = raise (NotFound "x")
//                      ^ d: src/Pricing.fs:41

let tariff = Tariff.tariffRate + Tariff.product [ 1 ]
//                  ^ d: src/Pricing.fs:46
//                                      ^ d: src/Pricing.fs:45

let named = Tariff.``returns "the" rate`` ()
//                   ^ d: src/Pricing.fs:44

let weighAll xs =
    let acc = List.sum xs
    acc
//  ^ d: src/Courier.fs:62
//  status: (local)

let faked = fakeRate + fakeVerbatim + fakeTriple
//          ^ d: none
//                     ^ d: none
//                                    ^ d: none
