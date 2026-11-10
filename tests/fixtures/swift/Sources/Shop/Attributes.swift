func escaping() {}

func discardableResult() {}

func Glow() {}

@MainActor
// ^ d: none
final class Gazette {
    @Published var ticks = 0
    // ^ d: none
    @Glow var lumen: Torch = Torch()
    // ^ d: Sources/Shop/Rebound.swift:20

    @available(iOS 15, *)
    // ^ d: none
    @discardableResult
    // ^ d: none
    func reckon(then done: @escaping () -> Void) -> Int {
        //                  ^ d: none
        done()
        return ticks
    }
}
