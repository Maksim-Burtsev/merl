struct Larder {
    static let stocked = Larder()

    static func open() -> Self {
        return .stocked
//              ^ d: Sources/Shop/Larder.swift:2
    }
}

protocol Stocked {
    static var reserve: Self { get }
}

func restock<T: Stocked>() -> T {
    return .reserve
//          ^ d: Sources/Shop/Larder.swift:11
}

extension Locale {
    static let pantry = Locale(identifier: "en")

    static func shelved() -> Self {
        return .pantry
//              ^ d: Sources/Shop/Larder.swift:20
    }
}
