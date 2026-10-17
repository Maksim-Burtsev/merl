let menu = """
    func whistle() {}
    func strain() {}
    struct Lid {}
    var celsius: Int { 0 }
    """

let brewMenu = #"""
    func pour(into cup: Cup) {}
    extension Kettle { func lift() {} }
    """#

func serve(_ pot: Teapot, _ kettle: Kettle, _ brewing: any Brewing) {
    _ = pot.steep()
    //      ^ d: picker Sources/Shop/Tea/Kettle.swift:7, Sources/Shop/Tea/Kettle.swift:12, Sources/Shop/Tea/Kettle.swift:31; want Sources/Shop/Tea/Kettle.swift:12 (#984)
    pot.rinse()
    //  ^ d: Sources/Shop/Tea/Teapot.swift:14
    pot.strain()
    //  ^ d: Sources/Shop/Tea/Kettle.swift:14
    _ = kettle.steep()
    //         ^ d: Sources/Shop/Tea/Kettle.swift:31
    _ = brewing.steep()
    //          ^ d: picker Sources/Shop/Tea/Kettle.swift:7, Sources/Shop/Tea/Kettle.swift:12, Sources/Shop/Tea/Kettle.swift:31; want Sources/Shop/Tea/Kettle.swift:7 (#984)
    kettle.whistle()
    //     ^ d: Sources/Shop/Tea/Kettle.swift:41
    _ = Kettle.fresh()
    //         ^ d: Sources/Shop/Tea/Kettle.swift:42
    _ = Lid(vents: 1)
    //  ^ d: Sources/Shop/Tea/Kettle.swift:2
    _ = Kettle.Lid(knob: 1)
    //         ^ d: Sources/Shop/Tea/Kettle.swift:18
    kettle.lid.lift()
    //         ^ d: Sources/Shop/Tea/Kettle.swift:46
    kettle.pour(over: Saucer())
    //     ^ d: picker Sources/Shop/Tea/Kettle.swift:33, Sources/Shop/Tea/Kettle.swift:34, Sources/Shop/Tea/Kettle.swift:35; want Sources/Shop/Tea/Kettle.swift:34 (#983)
    kettle.pour(into: Cup(from: Saucer()))
    //     ^ d: picker Sources/Shop/Tea/Kettle.swift:33, Sources/Shop/Tea/Kettle.swift:34, Sources/Shop/Tea/Kettle.swift:35; want Sources/Shop/Tea/Kettle.swift:33 (#983)
    //                ^ d: Sources/Shop/Tea/Teapot.swift:5; want Sources/Shop/Tea/Teapot.swift:8 (#983)
    kettle.pour(3)
    //     ^ d: picker Sources/Shop/Tea/Kettle.swift:33, Sources/Shop/Tea/Kettle.swift:34, Sources/Shop/Tea/Kettle.swift:35; want Sources/Shop/Tea/Kettle.swift:35 (#983)
    _ = Cup(ounces: 2)
    //  ^ d: Sources/Shop/Tea/Teapot.swift:5; want Sources/Shop/Tea/Teapot.swift:7 (#983)
    _ = kettle.celsius
    //         ^ d: Sources/Shop/Tea/Kettle.swift:23
    kettle.fahrenheit = 90
    //     ^ d: Sources/Shop/Tea/Kettle.swift:24
    _ = menu
    //  ^ d: Sources/Shop/Tea/Table.swift:1
}

func pour(into cup: Cup) {}

func lay(_ kettle: Kettle?) {
    kettle?.boil { [kettle] in
        kettle?.whistle()
        //^ d: Sources/Shop/Tea/Table.swift:53; want Sources/Shop/Tea/Table.swift:54 (#986)
    }
    guard let kettle else { return }
    kettle.whistle()
    // ^ d: Sources/Shop/Tea/Table.swift:53; want Sources/Shop/Tea/Table.swift:58 (#986)
    pour(into: Cup(ounces: 3))
    // ^ d: Sources/Shop/Tea/Table.swift:51
    kettle.pour(into: Cup(ounces: 4))
    //     ^ d: picker Sources/Shop/Tea/Kettle.swift:33, Sources/Shop/Tea/Kettle.swift:34, Sources/Shop/Tea/Kettle.swift:35; want Sources/Shop/Tea/Kettle.swift:33 (#983)
}
