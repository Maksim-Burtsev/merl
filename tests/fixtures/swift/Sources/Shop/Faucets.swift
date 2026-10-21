final class Faucet {
    func pour(level: Int) -> Faucet { self }
}

final class Cistern {
    func pour(level: Int) -> Cistern { self }
}

final class Plumbing {
    func faucet(_ level: Int) -> Faucet { Faucet() }
    func faucet(_ name: String,
                hot: Bool = false,
                cold: Bool = false,
                level: Int = 0,
                drip: Bool = false,
                spout: String = "",
                tap: String = "",
                valve: String = "",
                washer: String = "") -> Faucet { Faucet() }
    func either(_ level: Int) -> Faucet { Faucet() }
    func either(_ name: String) -> Cistern { Cistern() }
}

class PlumbingCase {
    func kept(_ plumbing: Plumbing) -> Plumbing { plumbing }
}

final class FaucetCase: PlumbingCase {
    func testPour() {
        let plumbing = kept(Plumbing())
        plumbing.faucet(1)
            .pour(level: 2)
        //   ^ d: Sources/Shop/Faucets.swift:2
        //     status: via plumbing.faucet() -> Faucet
        plumbing.faucet(1).pour(level: 2)
        //                 ^ d: Sources/Shop/Faucets.swift:2
        plumbing.either(1)
            .pour(level: 2)
        //   ^ d: !jump
    }
}
