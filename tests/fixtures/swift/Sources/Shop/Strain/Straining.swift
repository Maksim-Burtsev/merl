// A `case let x as T:` arm of a switch types x as T inside the arm; outside it, or bound by any
// other pattern, the name keeps its own type (#1494).
func straining(_ s: Strainer, _ c: Colander, _ any: Any) -> Int {
    var total = 0
    switch s {
    case let e as Tamis:
        total += e.strain()
        //         ^ d: Sources/Shop/Strain/Tamis.swift:2
    case let e as Colander:
        total += e.meshCount()
        //         ^ d: Sources/Shop/Strain/Colander.swift:3
    case var e as Tamis where total > 1:
        total += e.strain()
        //         ^ d: Sources/Shop/Strain/Tamis.swift:2
        e = Tamis()
    default:
        total += s.strain()
        //         ^ d: !jump
    }
    switch any {
    case let (e, f) as (Tamis, Colander):
        total += e.strain() + f.strain()
        //         ^ d: picker Sources/Shop/Strain/Colander.swift:2, Sources/Shop/Strain/Strainer.swift:2, Sources/Shop/Strain/Tamis.swift:2, Sources/Shop/Tea/Kettle.swift:14; want Sources/Shop/Strain/Tamis.swift:2 (#1494)
    case let e as Tamis?:
        total += e?.strain() ?? 0
        //          ^ d: Sources/Shop/Strain/Tamis.swift:2
    default:
        break
    }
    let e = c
    total += e.strain()
    //         ^ d: Sources/Shop/Strain/Colander.swift:2
    return total
}
