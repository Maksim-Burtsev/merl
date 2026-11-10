class Dipper {
    func dip() -> Int {
        return 1
    }

    func size(_ n: Int) -> Self {
        return self
    }

    static func builder() -> DipperBuilder {
        return DipperBuilder()
    }
}

class DipperBuilder {
    func size(_ n: Int) -> DipperBuilder {
        return self
    }

    func build() -> Dipper {
        return Dipper()
    }
}

class Scoop {
    func dip() -> Int {
        return 2
    }

    func build() -> Scoop {
        return self
    }

    func size(_ n: Int) -> Scoop {
        return self
    }
}

protocol Pouring {
    static func builder() -> Scoop
}

struct Vault<Dipper: Pouring> {
    let racks: Int
}
