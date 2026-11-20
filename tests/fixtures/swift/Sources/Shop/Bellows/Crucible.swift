class Crucible {
    func melt(_ check: @escaping (Int) -> Bool) -> Self { self }
    func skim(_ done: @escaping (Int) -> Void) -> Self { self }
}

extension Crucible {
    func melt() -> Self { self }
}
