class Flagon {
    func decant(_ n: Int) {}
}

class Siphon: Flagon {
    func decant(into n: Int) {}

    func go() {
        self.decant(1)
        //   ^ d: Sources/Shop/Decant/Overrides.swift:2
        decant(2)
        // ^ d: Sources/Shop/Decant/Overrides.swift:2
        self.decant(into: 3)
        //   ^ d: Sources/Shop/Decant/Overrides.swift:6
    }
}

class Tankard: Flagon {
    func decant(_ s: String) {}

    func go() {
        self.decant(1)
        //   ^ d: Sources/Shop/Decant/Overrides.swift:2
        decant(1)
        // ^ d: Sources/Shop/Decant/Overrides.swift:2
        self.decant("x")
        //   ^ d: Sources/Shop/Decant/Overrides.swift:19
        decant(true)
        // ^ d: Sources/Shop/Decant/Overrides.swift:19
    }
}

class Pitcher: Flagon {
    override func decant(_ n: Int) {}
    func decant(_ s: String) {}

    func go() {
        self.decant(1)
        //   ^ d: Sources/Shop/Decant/Overrides.swift:34
        decant("x")
        // ^ d: Sources/Shop/Decant/Overrides.swift:35
    }
}

class Mug: Flagon {}

class Stein: Mug {
    func decant(_ s: String) {}

    func go() {
        self.decant(1)
        //   ^ d: Sources/Shop/Decant/Overrides.swift:2
    }
}

func pourAll(s: Siphon, t: Tankard, n: Int) {
    s.decant(1)
    // ^ d: Sources/Shop/Decant/Overrides.swift:2
    s.decant(into: 1)
    // ^ d: Sources/Shop/Decant/Overrides.swift:6
    t.decant(1)
    // ^ d: Sources/Shop/Decant/Overrides.swift:2
    t.decant("x")
    // ^ d: Sources/Shop/Decant/Overrides.swift:19
    t.decant(n)
    // ^ d: Sources/Shop/Decant/Overrides.swift:2
    Tankard().decant(2)
    //        ^ d: Sources/Shop/Decant/Overrides.swift:2
}

class Ladle: Flagon {
    func decant(_ body: () -> Void) {}

    func go() {
        decant {}
        // ^ d: Sources/Shop/Decant/Overrides.swift:72
    }
}

class Spout: NSObject {
    func decant(_ s: String) {}

    func go() {
        decant(1)
        // ^ d: Sources/Shop/Decant/Overrides.swift:81
    }
}

class Jerrycan: Flagon {
    func decant<T>(_ t: T) {}
    func decant(_ s: String?) {}

    func go() {
        decant(1)
        // ^ d: picker Sources/Shop/Decant/Overrides.swift:90, Sources/Shop/Decant/Overrides.swift:91
    }
}

class Demijohn: Flagon {
    func decant(_ s: String?) {}

    func go() {
        self.decant(1)
        //   ^ d: Sources/Shop/Decant/Overrides.swift:2
    }
}
