extension SousedLater {
    func souse(_ f: (Tun) -> Void) {}
}

extension SousedConforming: CustomStringConvertible {
    var description: String { "" }
}

extension SousedSnug { func souse(_ f: (Tun) -> Void) {} }

typealias SousedNick = SousedNicked
extension SousedNick: CustomStringConvertible {
    var description: String { "" }
}

extension Shop.SousedQualified: CustomStringConvertible {
    var description: String { "" }
}

@available(*, deprecated, message: "use f()") extension SousedMarked: CustomStringConvertible {
    var description: String { "" }
}
