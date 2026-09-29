public struct Courier {
    public let name: String
    public init(name: String) {
        self.name = name
    }
}

public func weigh(_ grams: Int) -> Int {
    grams / 1000
}

public enum Depot {
    public static func open() -> Bool {
        true
    }
}

extension String {
    public var parcel: String { self }
}
