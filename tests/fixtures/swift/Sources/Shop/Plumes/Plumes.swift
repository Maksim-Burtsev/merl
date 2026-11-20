// A closure's own `plumes` hides the parameter of that name (#1610).
class Plume {
    func scrawl() {}
}

class Blotter {
    func scrawl() {}
}

func writeAll(plumes: [Plume], stacks: [[Blotter]]) {
    stacks.forEach { plumes in _ = plumes[0].scrawl() }
    //                                       ^ d: !jump
}
