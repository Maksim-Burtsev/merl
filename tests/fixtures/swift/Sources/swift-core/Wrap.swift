@_exported import Warehouse
@propertyWrapper
public struct Wrap {
    public var wrappedValue: Int
    public init(wrappedValue: Int) { self.wrappedValue = wrappedValue }
}
