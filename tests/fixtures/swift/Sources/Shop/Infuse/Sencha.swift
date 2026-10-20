class Sencha {
    var grade: Int
    var name: String

    init(grade: Int) {
        self.grade = grade
        self.name = ""
    }

    func uppercased() -> String { name }
    func append(_ c: Character) { name.append(c) }
    func label() -> String { "sencha" }
    func steeped() -> Sencha { self }
    class func make() -> Sencha { Sencha(grade: 0) }
}

final class Gyokuro: Sencha {
    override func label() -> String { "gyokuro" }
    func veil() -> Int { grade }
}

protocol Infusing {
    func minutes() -> Int
    func alarm() -> Int
}

extension Infusing {
    func minutes() -> Int { 3 }
    func alarm() -> Int { minutes() * 60 }
}

extension Sencha: Infusing {
    func alarm() -> Int { 90 }
}

extension StringProtocol {
    func sniffed() -> Int { count }
}

struct Whisk {
    var scoops = 0
    var cups = 0

    func sift(_ n: Int) -> Whisk { Whisk(scoops: n, cups: cups) }
    func fill(_ n: Int) -> Whisk { Whisk(scoops: scoops, cups: n) }
    func finish() -> Bowl { Bowl(froth: scoops * cups) }
}

struct Bowl {
    let froth: Int
    func taste() -> Int { froth }
}

func slurp() -> Int { 1 }

#if os(macOS)
typealias Kyusu = Gyokuro
#else
typealias Kyusu = Sencha
#endif
