class BaseTestCase {
    let timeout: Double = 10
}

struct Config {
    let timeout: Int
}

final class DownloadTests: BaseTestCase {
    func testDownload() {
        wait(timeout)
        //   ^ d: Tests/ShopTests/BaseTestCase.swift:2
        //     status: timeout → BaseTestCase.timeout (via self: DownloadTests)
    }

    func wait(_ seconds: Double) {}
}
