enum Wrapping {
    case box
}

let wrapping: Wrapping = .box
//            ^ d: picker Sources/Shop/Visibility.swift:2, Tests/ShopTests/WrappingTests.swift:1

func testHamper() {
    struct Filling {}
    _ = Filling()
}
