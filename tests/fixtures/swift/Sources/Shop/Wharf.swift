// A static call followed by a property as an argument (#1341): the static func's written return
// type, then the property's written type, picks the overload. A generic return, an optional
// chain and a generic property keep the picker.
struct Wharf {
    static func berth(_ bay: Int) -> Wharf { Wharf() }
    static func make() -> Self { Self() }
    static func any<T>(_ value: T) -> T { value }
    static func maybe() -> Wharf? { nil }

    func next() -> Wharf { self }

    var bale: Bale { Bale() }
}

struct Crane<Load> {
    static func hoist(_ load: Load) -> Crane<Load> { Crane() }

    var load: Load? { nil }
}

func unload(hold: Hold, wharf: Wharf) {
    hold.load(Wharf.berth(1).bale)
//       ^ d: Sources/Shop/Overloads.swift:20
    let bale = Wharf.berth(2).bale
    hold.load(bale)
//       ^ d: Sources/Shop/Overloads.swift:20
    hold.load(Wharf.make().bale)
//       ^ d: Sources/Shop/Overloads.swift:20
    hold.load(wharf.next().bale)
//       ^ d: Sources/Shop/Overloads.swift:20
    hold.load(Wharf.berth(3).sack)
//       ^ d: Sources/Shop/Overloads.swift:21
    hold.load(Wharf.any(wharf).bale)
//       ^ d: picker Sources/Shop/Overloads.swift:20, Sources/Shop/Overloads.swift:21
    hold.load(Wharf.maybe()?.bale)
//       ^ d: picker Sources/Shop/Overloads.swift:20, Sources/Shop/Overloads.swift:21
    hold.load(Crane.hoist(Bale()).load)
//       ^ d: picker Sources/Shop/Overloads.swift:20, Sources/Shop/Overloads.swift:21
}
