// A test target's member shadows the library's one of the same signature, and a member read
// without a call is the property of its name, not the method (#1341).
extension Result {
    var isSettled: Bool {
        guard case .success = self else { return false }
        return true
    }
}

func settledHere() -> Bool {
    let outcome = Result { try brew() }
    return outcome.isSettled
    //             ^ d: Sources/Shop/Shadows.swift:4
}

extension Error {
    var asShopError: Int? { nil }

    func asShopError(or fallback: Int) -> Int { fallback }
}

func shopErrorCode() -> Int? {
    let outcome = Result { try brew() }
    return outcome.failure?.asShopError
    //                      ^ d: Sources/Shop/Shadows.swift:17
}

func shopErrorCodeOr() -> Int? {
    let outcome = Result { try brew() }
    return outcome.failure?.asShopError(or: 0)
    //                      ^ d: picker Sources/Shop/Shadows.swift:17, Sources/Shop/Shadows.swift:19
}
