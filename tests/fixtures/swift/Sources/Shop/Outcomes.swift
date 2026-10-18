// The night pass of 2026-10-19 over `d` in Swift (#1067, #1068, #1053, #1052, #1069, #1047, #1054,
// #1055), answers sourcekit-lsp's. An implicit `.x` is never an instance property (#1067).
extension Result {
    var success: Success? {
        guard case let .success(value) = self else { return nil }
        return value
    }
}

func settleOutcome(_ value: Int) -> Result<Int, Error> {
    return .success(value)
    //      ^ d: none
}

func outcomeValue(_ outcome: Result<Int, Error>) -> Int? {
    outcome.success
    //      ^ d: Sources/Shop/Outcomes.swift:4
}

func outcomeValueBelow(_ outcome: Result<Int, Error>) -> Int? {
    outcome
        .success
    //   ^ d: Sources/Shop/Outcomes.swift:4
}
