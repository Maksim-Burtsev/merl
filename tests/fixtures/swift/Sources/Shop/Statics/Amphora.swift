// #1489: a nested enum whose cases sit on its declaration line.
final class Amphora {
    enum Glaze { case crackle, matt }
}

final class Krater {
    enum Glaze { case crackle }
}

func glazeAll() {
    _ = Amphora.Glaze.crackle
    //                ^ d: Sources/Shop/Statics/Amphora.swift:3
    _ = Krater.Glaze.crackle
    //               ^ d: Sources/Shop/Statics/Amphora.swift:7
    _ = Amphora.Glaze.matt
    //                ^ d: Sources/Shop/Statics/Amphora.swift:3
}
