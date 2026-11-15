func doubled(_ prices: [Int]) -> [Int] {
    var out = [Int]()
    for p in prices {
        out.append(p * 2)
        //  ^ d: none
    }
    let text = "a,b"
    _ = text.contains("a")
    //       ^ d: none
    return out
}
