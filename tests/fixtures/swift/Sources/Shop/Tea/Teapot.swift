struct Leaf {
    let grade: Int
}

struct Cup {
    let ounces: Int
    init(ounces: Int) { self.ounces = ounces }
    init(from saucer: Saucer) { ounces = 0 }
}

struct Saucer {}

struct Teapot: Brewing {
    func rinse() {}
}
