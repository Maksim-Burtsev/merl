struct Churn {
    func stir(_ n: Int) -> ChurnMid { ChurnMid() }
}

struct ChurnMid {
    func curdle() -> ChurnEnd { ChurnEnd() }
}

struct ChurnEnd {
    func sample() -> Int { 1 }
}

struct Froth {
    func stir(_ n: Int) -> FrothMid { FrothMid() }
}

struct FrothMid {
    func curdle() -> FrothEnd { FrothEnd() }
}

struct FrothEnd {
    func sample() -> Int { 2 }
}

struct Pail<T> {
    func each(_ f: (T) -> Int) -> Int { 0 }
}

func nested(_ fs: Pail<Froth>, _ cs: Pail<Churn>) -> Int {
    let w = Froth()
    _ = fs.each { w in cs.each { w in w.stir(1).curdle().sample() } }
    //                                          ^ d: Sources/Shop/Infuse/Shadows.swift:6
    _ = fs.each { w in cs.each { w in w.stir(1).curdle().sample() } }
    //                                                   ^ d: picker Sources/Shop/Infuse/Shadows.swift:10, Sources/Shop/Infuse/Shadows.swift:22
    _ = cs.each { w in 0 } + fs.each { w in w.stir(1).curdle().sample() }
    //                                                         ^ d: picker Sources/Shop/Infuse/Shadows.swift:10, Sources/Shop/Infuse/Shadows.swift:22
    _ = cs.each { [w] v in w.stir(1).curdle().sample() }
    //                               ^ d: Sources/Shop/Infuse/Shadows.swift:18
    return cs.each { [w] in w.stir(1).curdle().sample() }
    //                                ^ d: Sources/Shop/Infuse/Shadows.swift:18
}

func shadowed(_ maybe: Froth?, _ fs: [Froth], _ opts: [Froth?], _ f: Froth) -> Int {
    let w = Churn()
    if let w = maybe { return w.stir(1).curdle().sample() }
    //                                           ^ d: !jump
    if case let w? = maybe { return w.stir(1).curdle().sample() }
    //                                                 ^ d: !jump
    for (_, w) in fs.enumerated() { return w.stir(1).curdle().sample() }
    //                                                        ^ d: !jump
    switch maybe {
    case .some(let w): return w.stir(1).curdle().sample()
    //                                           ^ d: !jump
    case .none:
        break
    }
    for case let w? in opts { return w.stir(1).curdle().sample() }
    //                                                  ^ d: !jump
    while let w = maybe { return w.stir(1).curdle().sample() }
    //                                              ^ d: !jump
    func inner(_ w: Froth) -> Int { w.stir(1).curdle().sample() }
    //                                                 ^ d: !jump
    if true { let w = f; return w.stir(1).curdle().sample() }
    //                                             ^ d: !jump
    return inner(f) + fs.map {
        w in w.stir(1).curdle().sample()
        //                      ^ d: !jump
    }.count
}

@propertyWrapper
struct Kept {
    var wrappedValue: Churn
    var projectedValue: Froth { Froth() }
}

struct Held {
    @Kept var w: Churn
    func go() -> Int {
        $w.stir(1).curdle().sample()
        //                  ^ d: !jump
    }
}

struct Crock<FrothEnd> {
    let v: FrothEnd
    func me() -> Crock<FrothEnd> { self }
}

extension Crock {
    func got() -> FrothEnd { v }
}

struct Strainer {
    func held<
        FrothEnd
    >(_ b: FrothEnd) -> FrothEnd { b }
    func kept<C: Collection<Int>, FrothEnd>(_ c: C, _ b: FrothEnd) -> FrothEnd { b }
    func again() -> Strainer { self }
}

struct Outer<FrothEnd> {
    struct Inner {
        func held(_ b: FrothEnd) -> FrothEnd { b }
    }
}

func generic(_ c: Crock<ChurnEnd>, _ s: Strainer, _ i: Outer<ChurnEnd>.Inner) -> Int {
    _ = c.me().got().sample()
    //               ^ d: !jump
    _ = s.again().held(ChurnEnd()).sample()
    //                             ^ d: !jump
    _ = s.again().kept([1], ChurnEnd()).sample()
    //                                  ^ d: !jump
    return i.held(ChurnEnd()).sample()
    //                        ^ d: !jump
}
