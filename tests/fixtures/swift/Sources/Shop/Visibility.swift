// What the compiler cannot see from the cursor is no candidate (#375).
enum Wrapping {
    case paper
}

func wrap() -> Wrapping {
    //         ^ d: Sources/Shop/Visibility.swift:2
    //           status: Wrapping: by name, 1 match
    .paper
}

public struct Hamper<Filling: Sendable> {
    typealias Unpack = (Filling) -> Void
    //                  ^ d: Sources/Shop/Visibility.swift:12
    //                    status: Filling: local
}

struct Prober {
    enum Reading {
        case clear
    }

    func last() -> Reading {
        //         ^ d: Sources/Shop/Visibility.swift:19
        .clear
    }
}

// A nested type is not seen bare outside its type.
func reread(_ r: Reading) {}
//               ^ d: none
//                 status: no definition for Reading

class Hatch {
    enum Latch {
        case shut
    }
}

final class TrapHatch: Hatch {
    func latch() -> Latch {
        //          ^ d: Sources/Shop/Visibility.swift:35
        .shut
    }
}

private func sealBox() -> Int {
    0
}

let sealed = sealBox()
//           ^ d: Sources/Shop/Visibility.swift:47
//             status: sealBox: by name, 1 match

func cushion() -> Int {
    struct Padding {
        let depth = 1
    }
    return Padding().depth
    //     ^ d: Sources/Shop/Visibility.swift:56
}

let padding: Padding? = nil
//           ^ d: none
