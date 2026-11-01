@testable import Shop

extension Result {
    var isSettled: Bool {
        guard case .success = self else { return false }
        return true
    }
}

func testSettled() {
    let outcome = Result { try brew() }
    _ = outcome.isSettled
    //          ^ d: Tests/ShopTests/ShadowTests.swift:4
}
