// The adversarial corner of #982 for Swift, its answers sourcekit-lsp's.
struct Lid {
    let vents: Int
}

protocol Brewing {
    func steep() -> Int
    func rinse()
}

extension Brewing {
    func steep() -> Int { 3 }
    func rinse() {}
    func strain() {}
}

final class Kettle: Brewing {
    struct Lid {
        let knob: Int
    }

    var water: Int = 0
    var celsius: Int { water * 2 }
    var fahrenheit: Int {
        get { celsius * 9 / 5 + 32 }
        set { water = newValue }
    }
    let lid = Lid(knob: 1)
    var leaf: Leaf?

    func steep() -> Int { celsius }

    func pour(into cup: Cup) {}
    func pour(over saucer: Saucer) {}
    func pour(_ amount: Int) {}

    func boil(then done: @escaping () -> Void) { done() }
}

extension Kettle {
    func whistle() {}
    static func fresh() -> Kettle { Kettle() }
}

extension Kettle.Lid {
    func lift() {}
}

extension Kettle {
    func warm() {
        boil { [weak self] in
            self?.whistle()
            //    ^ d: Sources/Shop/Tea/Kettle.swift:41
            guard let self else { return }
            self.whistle()
            //   ^ d: Sources/Shop/Tea/Kettle.swift:41
            _ = self.celsius
            //       ^ d: Sources/Shop/Tea/Kettle.swift:23
        }
        boil { [self] in
            self.pour(into: Cup(ounces: 1))
            //   ^ d: Sources/Shop/Tea/Kettle.swift:33
            self.pour(over: Saucer())
            //   ^ d: Sources/Shop/Tea/Kettle.swift:34
            pour(5)
            // ^ d: Sources/Shop/Tea/Kettle.swift:35
        }
        _ = lid.knob
        //      ^ d: Sources/Shop/Tea/Kettle.swift:19
        lid.lift()
        //  ^ d: Sources/Shop/Tea/Kettle.swift:46
        _ = Lid(knob: 2)
        //  ^ d: Sources/Shop/Tea/Kettle.swift:18
        _ = fahrenheit
        //  ^ d: Sources/Shop/Tea/Kettle.swift:24
        fahrenheit = 100
        // ^ d: Sources/Shop/Tea/Kettle.swift:24
        _ = steep()
        //  ^ d: Sources/Shop/Tea/Kettle.swift:31
        strain()
        // ^ d: Sources/Shop/Tea/Kettle.swift:14
        rinse()
        // ^ d: Sources/Shop/Tea/Kettle.swift:13
    }

    func sip(leaf: Leaf?) {
        guard let leaf else { return }
        _ = leaf.grade
        //  ^ d: Sources/Shop/Tea/Kettle.swift:87
        //       ^ d: Sources/Shop/Tea/Teapot.swift:2
        if let leaf = self.leaf {
            _ = leaf
            //  ^ d: Sources/Shop/Tea/Kettle.swift:91
        }
        _ = self.leaf
        //       ^ d: Sources/Shop/Tea/Kettle.swift:29
    }

    func taste(_ cups: [Cup]) -> [Int] {
        cups.map { water in water.ounces }
        //                  ^ d: Sources/Shop/Tea/Kettle.swift:100
        //                        ^ d: Sources/Shop/Tea/Teapot.swift:6
    }
}
