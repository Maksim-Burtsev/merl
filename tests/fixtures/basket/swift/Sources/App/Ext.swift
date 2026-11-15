@available(macOS 10.15, *) extension Array {
    func chunked(_ size: Int) -> [[Element]] {
        [self]
    }
}

extension String {
    func shout() -> String {
        uppercased()
    }
}
