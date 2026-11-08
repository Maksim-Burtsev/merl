struct Goblet {
    func quaff() {}
    func clink() {}
}

protocol Toaster {
    func clink()
}

struct Rack<Goblet> {
    var held: Goblet
    func use(_ x: Goblet) {
        x.quaff()
        //^ d: none
        held.quaff()
        //   ^ d: Sources/Shop/Goblets.swift:2; want none (#1438)
    }
}

func fill<Goblet>(_ x: Goblet) {
    x.quaff()
    //^ d: none
}

func unbound<T>(_ x: T) {
    x.quaff()
    //^ d: none
}

func bounded<T: Toaster>(_ x: T) {
    x.clink()
    //^ d: picker Sources/Shop/Goblets.swift:3, Sources/Shop/Goblets.swift:7
}

func wrapped<Goblet>(
    _ x: Goblet
) {
    x.quaff()
    //^ d: none
}

func plain(_ x: Goblet) {
    x.quaff()
    //^ d: Sources/Shop/Goblets.swift:2
}

struct Rack3<T> {
    var held: T
    func use() {
        self.held.quaff()
        //        ^ d: Sources/Shop/Goblets.swift:2; want none (#1438)
    }
}

struct Rack4<Goblet> {
    var held: Goblet
    func use() {
        self.held.quaff()
        //        ^ d: Sources/Shop/Goblets.swift:2; want none (#1438)
    }
}
