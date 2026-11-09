class Gridiron {
    func sear() -> Int { 2 }

    func scoped(_ items: [Int]) -> [Int] {
        let a = items.map { _ in self.sear() }
        //                            ^ d: Sources/Shop/Thisbound/Gridiron.swift:2
        let b = items.map { [unowned self] _ in sear() }
        //                                      ^ d: Sources/Shop/Thisbound/Gridiron.swift:2
        return a + b
    }
}
