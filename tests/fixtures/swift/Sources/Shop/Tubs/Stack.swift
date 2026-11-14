class Stack<T> {
    func max(_ by: (T) -> Int) -> T? { nil }
    func sorted(_ by: (T) -> Int) -> [T] { [] }
}

class StackUse {
    let s: Stack<Tub> = Stack<Tub>()
    func a() { _ = s.max { $0.slop() } }
    //                        ^ d: Sources/Shop/Tubs/Tub.swift:3
    func b() { _ = s.sorted { $0.slop() } }
    //                           ^ d: Sources/Shop/Tubs/Tub.swift:3
}
