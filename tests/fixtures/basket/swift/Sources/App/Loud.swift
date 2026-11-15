func loud() -> Int {
    let xs = [Int]()
    _ = xs.chunked(2)
    //     ^ d: swift/Sources/App/Ext.swift:2
    let s = "a"
    _ = s.shout()
    //    ^ d: swift/Sources/App/Ext.swift:8
    return xs.count
}
