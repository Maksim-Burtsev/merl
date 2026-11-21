class Bellows {
    func blow(_ fuel: String, hot: Bool = false) -> Crucible { Crucible() }
    func blow(_ fuel: Int) -> Crucible { Crucible() }
    func puff(_ fuel: Int) -> Crucible { Crucible() }
    func puff(_ fuel: String) -> Smelter { Smelter() }
}

extension Bellows {
    func blow(for slag: Int) -> Smelter? { nil }
}
