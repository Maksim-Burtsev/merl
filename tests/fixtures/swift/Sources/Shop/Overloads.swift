import Foundation

// Overloads that differ only by the argument's type (#1341): an argument whose type is plain
// (a literal, a value of a written type, a constructor call) and exactly one overload's parameter
// that takes it, the others a struct or enum of another name. A protocol, a generic, a class, an
// alias, two numeric literals and the same type twice keep the picker.
struct Bale {}
struct Sack {}
final class Cask {}
final class Keg {}
protocol Stowable {}
extension Bale: Stowable {}
typealias Tote = Bale

final class Hold {
    func stow(_ stream: InputStream, in bay: Int) {}
    func stow(_ data: Data, in bay: Int) {}
    func stow(_ url: URL, in bay: Int) {}

    func load(_ crate: Bale) {}
    func load(_ barrel: Sack) {}

    func tag(_ name: String) {}
    func tag(_ data: Data) {}

    func weigh(_ kilos: Int) {}
    func weigh(_ kilos: Double) {}

    func pack(_ crate: Bale) {}
    func pack(_ item: any Stowable) {}

    func lift<T>(_ thing: T) {}
    func lift(_ data: Data) {}

    func roll(_ cask: Cask) {}
    func roll(_ keg: Keg) {}

    func ship(_ crate: Bale, rush: Bool = false) {}
    func ship(_ crate: Bale) {}

    func send(_ stream: InputStream) {
        stow(stream, in: 1)
//      ^ d: Sources/Shop/Overloads.swift:16
        let data = Data("crate".utf8)
        stow(data, in: 2)
//      ^ d: Sources/Shop/Overloads.swift:17
        let crate: Bale = Bale()
        load(crate)
//      ^ d: Sources/Shop/Overloads.swift:20
        load(Sack())
//      ^ d: Sources/Shop/Overloads.swift:21
        tag("label")
//      ^ d: Sources/Shop/Overloads.swift:23
        weigh(3)
//      ^ d: picker Sources/Shop/Overloads.swift:26, Sources/Shop/Overloads.swift:27
        pack(crate)
//      ^ d: picker Sources/Shop/Overloads.swift:29, Sources/Shop/Overloads.swift:30
        lift(URL(fileURLWithPath: "/"))
//      ^ d: picker Sources/Shop/Overloads.swift:32, Sources/Shop/Overloads.swift:33
        let cask: Cask = Cask()
        roll(cask)
//      ^ d: picker Sources/Shop/Overloads.swift:35, Sources/Shop/Overloads.swift:36
        let tote: Tote = Tote()
        load(tote)
//      ^ d: picker Sources/Shop/Overloads.swift:20, Sources/Shop/Overloads.swift:21
        ship(crate)
//      ^ d: picker Sources/Shop/Overloads.swift:38, Sources/Shop/Overloads.swift:39
    }
}

func dock(hold: Hold) {
    let crate = Bale()
    hold.load(crate)
//       ^ d: Sources/Shop/Overloads.swift:20
    hold.tag(Data())
//       ^ d: Sources/Shop/Overloads.swift:24
}

struct Tallyman {
    func mark(_ done: Bool) {}
    func mark(_ note: String) {}

    func close() {
        mark("open" == "shut")
//      ^ d: picker Sources/Shop/Overloads.swift:80, Sources/Shop/Overloads.swift:81
        mark("shut")
//      ^ d: Sources/Shop/Overloads.swift:81
    }
}
