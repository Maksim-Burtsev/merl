class Smelter {
    func melt() -> Self { self }
    func skim(_ done: @escaping (Int) -> Void) -> Self { self }
}
