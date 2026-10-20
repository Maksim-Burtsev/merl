// The third adversarial corner of #1126 for Swift, its answers sourcekit-lsp's.
func stdValues(_ sencha: Sencha) -> String {
    let name = "oolong"
    _ = sencha.name.uppercased()
    //              ^ d: none
    var cup = [Int]()
    cup.append(1)
    //  ^ d: Sources/Shop/Infuse/Sencha.swift:11; want none (#1068)
    sencha.append("a")
    //     ^ d: Sources/Shop/Infuse/Sencha.swift:11
    return name.uppercased() + sencha.uppercased()
    //          ^ d: Sources/Shop/Infuse/Sencha.swift:10; want none (#1068)
    //                                ^ d: Sources/Shop/Infuse/Sencha.swift:10
}

func shadowOwnLine(_ sencha: Sencha) -> String {
    let sencha = sencha.steeped()
    //           ^ d: Sources/Shop/Infuse/Infusion.swift:16
    //                  ^ d: Sources/Shop/Infuse/Sencha.swift:13
    return sencha.label()
    //            ^ d: Sources/Shop/Infuse/Sencha.swift:12
}

func builder() -> Int {
    Whisk()
        .sift(2)
//       ^ d: Sources/Shop/Infuse/Sencha.swift:44
        .fill(3)
//       ^ d: Sources/Shop/Infuse/Sencha.swift:45
        .finish()
//       ^ d: Sources/Shop/Infuse/Sencha.swift:46
        .taste()
//       ^ d: Sources/Shop/Infuse/Sencha.swift:51
}

func onAResult() -> Int {
    Gyokuro(grade: 2).veil() + Sencha(grade: 1).minutes() + Gyokuro.make().alarm()
    //                ^ d: Sources/Shop/Infuse/Sencha.swift:19
    //                                          ^ d: Sources/Shop/Infuse/Sencha.swift:28
    //                                                              ^ d: Sources/Shop/Infuse/Sencha.swift:14
    //                                                                     ^ d: Sources/Shop/Infuse/Sencha.swift:33
}

func overridden(_ g: Gyokuro, _ s: Sencha) -> String {
    g.label() + s.label() + "\(g.grade)"
    //^ d: Sources/Shop/Infuse/Sencha.swift:18
    //            ^ d: Sources/Shop/Infuse/Sencha.swift:12
    //                           ^ d: Sources/Shop/Infuse/Sencha.swift:2
}

func inherited(_ g: Gyokuro) -> Int {
    g.alarm() + g.minutes()
    //^ d: Sources/Shop/Infuse/Sencha.swift:33
    //            ^ d: Sources/Shop/Infuse/Sencha.swift:28
}

func extended() -> Int {
    "oolong".sniffed()
    //       ^ d: Sources/Shop/Infuse/Sencha.swift:37
}

func loopHeader(_ leaves: [Sencha]) -> Int {
    var total = 0
    for (grade, sencha) in leaves.enumerated() {
        total += sencha.grade + grade
        //              ^ d: Sources/Shop/Infuse/Sencha.swift:2
        //                      ^ d: Sources/Shop/Infuse/Infusion.swift:64
    }
    return total
}

func destructured(_ pair: (Sencha, Int)) -> Int {
    let (sencha, grade) = pair
    return sencha.grade + grade
    //            ^ d: Sources/Shop/Infuse/Sencha.swift:2
    //                    ^ d: Sources/Shop/Infuse/Infusion.swift:73
}

func nested() -> Int {
    func slurp() -> Int { 5 }
    return slurp()
    //     ^ d: Sources/Shop/Infuse/Infusion.swift:80
}

func closureParam() -> Int {
    let gulp = { (slurp: Int) in slurp + 1 }
    return gulp(slurp())
    //     ^ d: Sources/Shop/Infuse/Infusion.swift:86
    //          ^ d: Sources/Shop/Infuse/Sencha.swift:54
}

func shorthand(_ leaves: [Sencha]) -> [String] {
    leaves.map { $0.label() }
    //              ^ d: Sources/Shop/Infuse/Sencha.swift:12
}

extension Gyokuro {
    func again() -> String {
        super.label() + label()
        //    ^ d: Sources/Shop/Infuse/Sencha.swift:12
        //              ^ d: Sources/Shop/Infuse/Sencha.swift:18
    }
}

func pot(_ kyusu: Kyusu) -> String {
    kyusu.label()
    //    ^ d: picker Sources/Shop/Infuse/Sencha.swift:12, Sources/Shop/Infuse/Sencha.swift:18
}

let recipe = """
    func slurp() -> Int { 0 }
    class Sencha {}
    \(slurp())
    """

/* func label() -> String { "" }
   struct Bowl {} */
func sipped() -> Int {
    slurp() + Bowl(froth: 1).taste()
//  ^ d: Sources/Shop/Infuse/Sencha.swift:54
    //        ^ d: Sources/Shop/Infuse/Sencha.swift:49
    //                       ^ d: Sources/Shop/Infuse/Sencha.swift:51
}

func neighbours(_ sencha: Sencha) -> Int {
    let total = sencha.steeped().label().count + Bowl(froth: 2).froth
    //                           ^ d: Sources/Shop/Infuse/Sencha.swift:12
    //                                   ^ d: none
    //                                                          ^ d: Sources/Shop/Infuse/Sencha.swift:50
    return total + Whisk().sift(1).finish().taste()
    //                     ^ d: Sources/Shop/Infuse/Sencha.swift:44
    //                             ^ d: Sources/Shop/Infuse/Sencha.swift:46
    //                                      ^ d: Sources/Shop/Infuse/Sencha.swift:51
}

func maybe(_ leaves: [Sencha]?) -> Int? {
    leaves.map { $0.count }
    //              ^ d: !jump
}

func kettle() -> Whisk { Whisk() }

struct Hearth {
    func kettle() -> Hearth { self }
    var scoops = 0
    func sift(_ n: Int) -> Hearth { self }
    func pour() -> Int {
        kettle().sift(1).scoops
        //               ^ d: Sources/Shop/Infuse/Infusion.swift:145
    }
}
