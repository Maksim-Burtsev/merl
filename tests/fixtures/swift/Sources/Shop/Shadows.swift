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

// An unapplied method of the receiver's own type is no other type's property of its name.
final class Gong {
    func resound() {}
}

struct Knell {
    var resound: Int = 0
}

func gongHandler(_ gong: Gong) -> () -> Void {
    let h = gong.resound
    //           ^ d: Sources/Shop/Shadows.swift:36
    return h
}

func gongHandlers(_ gongs: [String: Gong]) {
    let h = gongs["a"]?.resound
    //                  ^ d: picker Sources/Shop/Shadows.swift:36, Sources/Shop/Shadows.swift:40
    _ = h
}
