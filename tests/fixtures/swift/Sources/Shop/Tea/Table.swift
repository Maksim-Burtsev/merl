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
    //      ^ d: Sources/Shop/Tea/Kettle.swift:12
    pot.rinse()
    //  ^ d: Sources/Shop/Tea/Teapot.swift:14
    pot.strain()
    //  ^ d: Sources/Shop/Tea/Kettle.swift:14
    _ = kettle.steep()
    //         ^ d: Sources/Shop/Tea/Kettle.swift:31
    _ = brewing.steep()
    //          ^ d: Sources/Shop/Tea/Kettle.swift:7
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
    //     ^ d: Sources/Shop/Tea/Kettle.swift:34
    kettle.pour(into: Cup(from: Saucer()))
    //     ^ d: Sources/Shop/Tea/Kettle.swift:33
    //                ^ d: Sources/Shop/Tea/Teapot.swift:8
    kettle.pour(3)
    //     ^ d: Sources/Shop/Tea/Kettle.swift:35
    _ = Cup(ounces: 2)
    //  ^ d: Sources/Shop/Tea/Teapot.swift:7
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
        //^ d: Sources/Shop/Tea/Table.swift:54
    }
    guard let kettle else { return }
    kettle.whistle()
    // ^ d: Sources/Shop/Tea/Table.swift:58
    pour(into: Cup(ounces: 3))
    // ^ d: Sources/Shop/Tea/Table.swift:51
    kettle.pour(into: Cup(ounces: 4))
    //     ^ d: Sources/Shop/Tea/Kettle.swift:33
}

func pourLater(_ kettle: Kettle?) {
    kettle?.boil { [weak kettle] in
        kettle?.whistle()
        //^ d: Sources/Shop/Tea/Table.swift:68
        //      ^ d: Sources/Shop/Tea/Kettle.swift:41
    }
    if let kettle {
        kettle.whistle()
        //^ d: Sources/Shop/Tea/Table.swift:73
        //     ^ d: Sources/Shop/Tea/Kettle.swift:41
    }
}
