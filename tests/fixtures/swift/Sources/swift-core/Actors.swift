public final class MainActor {}

@propertyWrapper
public struct Published {
    public var wrappedValue: Int
    public init(wrappedValue: Int) { self.wrappedValue = wrappedValue }
}

public func available() {}
