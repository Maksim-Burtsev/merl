struct Basket {
    var items: [String] = []

    mutating func append(_ item: String) {
        items.append(item)
    }

    func contains(_ item: String) -> Bool {
        items.contains(item)
    }
}
