@propertyWrapper
public struct Boxed {
    public var wrappedValue: Int
    public init(wrappedValue: Int) { self.wrappedValue = wrappedValue }
}
