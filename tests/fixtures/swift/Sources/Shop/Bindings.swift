// The names a Swift line binds, and the blocks that own a local (#371, #366).
enum Parcel {
    case boxed(Int, Int)
    case tagged(label: Int)
}

let girth = 0
let crown = 0
let brand = 0
let heft = 0
let wick = 0
let flood = 0
let crest = 0

func unpack(_ parcel: Parcel, _ thing: Any, _ next: Int?) {
    let crown = 1
    switch parcel {
    case .boxed(let girth, crown):
        _ = girth
        //  ^ d: Sources/Shop/Bindings.swift:18
        _ = crown
        //  ^ d: Sources/Shop/Bindings.swift:16
    case .tagged(label: let brand):
        _ = brand
        //  ^ d: Sources/Shop/Bindings.swift:23
    }
    switch thing {
    case let heft as Int:
        _ = heft
        //  ^ d: Sources/Shop/Bindings.swift:28
    default:
        break
    }
    let wick: Int = 2
    _ = wick
    //  ^ d: Sources/Shop/Bindings.swift:34
    var ebb = 0, flood = 9
    _ = flood
    //  ^ d: Sources/Shop/Bindings.swift:37
    if ebb > 0,
       let crest = next {
        _ = crest
        //  ^ d: Sources/Shop/Bindings.swift:41
    }
    ebb += 1
}

let soot = 0
let slag = 0
let ash = 0
let cinder = 0
let flint = 0
let bellow = 0

final class Furnace {
    var flue = 0
    var grate = 0

    init() {
        let soot = 1
        _ = soot
    }

    subscript(i: Int) -> Int {
        let slag = i
        return slag
    }

    deinit {
        let ash = 0
        _ = ash
    }

    var heat: Int {
        get {
            let flue = 1
            _ = grate
            //  ^ d: Sources/Shop/Bindings.swift:57
            return flue
        }
        set {
            let grate = newValue
            _ = flue
            //  ^ d: Sources/Shop/Bindings.swift:56
            _ = grate
        }
    }

    var level = 0 {
        willSet {
            let cinder = newValue
            _ = cinder
        }
        didSet {
            let flint = oldValue
            _ = flint
        }
    }

    class func bellows() -> Int {
        let bellow = 1
        return bellow
    }

    func stoke() {
        _ = soot
        //  ^ d: Sources/Shop/Bindings.swift:48
        _ = slag
        //  ^ d: Sources/Shop/Bindings.swift:49
        _ = ash
        //  ^ d: Sources/Shop/Bindings.swift:50
        _ = cinder
        //  ^ d: Sources/Shop/Bindings.swift:51
        _ = flint
        //  ^ d: Sources/Shop/Bindings.swift:52
        _ = bellow
        //  ^ d: Sources/Shop/Bindings.swift:53
    }
}

