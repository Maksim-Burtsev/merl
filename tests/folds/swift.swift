// Every construct `f` folds in Swift, and the cases that once broke it.
@testable  // f: 2-6
import Alamofire  // f: 2-6
@_spi(Web) import Foundation  // f: 2-6

import XCTest  // f: 2-6

public typealias Handler = @Sendable (_ request: URLRequest,  // f: 8-9
                                      _ completion: @escaping (Int) -> Void) -> Void  // f: 8-9

protocol Shape {  // f: 11-14
    func area() -> Double  // f: 11-14
    var name: String { get }  // f: 11-14
}  // f: 11-14

enum Kind: Int {  // f: 16-21
    case a  // f: 16-21
    case b(  // f: 16-21
        Int  // f: 16-21
    )  // f: 16-21
}  // f: 16-21

final class Box<T>: NSObject, Shape where T: Equatable {  // f: 23-140
    @available(iOS 13,  // f: 23-140
               *)  // f: 23-140
    var count: Int = 0 {  // f: 26-30
        didSet {  // f: 27-29
            changed()  // f: 27-29
        }  // f: 27-29
    }  // f: 26-30

    var name: String {  // f: 32-39
        get {  // f: 33-35
            "box"  // f: 33-35
        }  // f: 33-35
        set(newValue) {  // f: 36-38
            store(newValue)  // f: 36-38
        }  // f: 36-38
    }  // f: 32-39

    init?(  // f: 23-140
        value: T  // f: 23-140
    ) {  // f: 43-45
        super.init()  // f: 43-45
    }  // f: 43-45

    func area() -> Double {  // f: 47-139
        let total = items  // f: 48-49
            .filter { $0 > 1 }  // f: 47-139
            .map {  // f: 50-52
                $0 * 2  // f: 47-139
            }  // f: 47-139
        let text = """  // f: 47-139
            { not a block  // f: 47-139
            \(names["}"] ?? "")  // f: 47-139
            """  // f: 47-139
        let raw = #"a "quoted" } brace"#  // f: 47-139
        if total.isEmpty {  // f: 58-64
            return 0  // f: 47-139
        } else if total.count == 1 {  // f: 60-64
            return 1  // f: 47-139
        } else {  // f: 47-139
            return 2  // f: 47-139
        }  // f: 47-139
        guard let first = total.first else {  // f: 65-67
            return 0  // f: 47-139
        }  // f: 47-139
        for x in total where x > first {  // f: 68-70
            print(x)  // f: 47-139
        }  // f: 47-139
        while running() {  // f: 71-73
            step()  // f: 47-139
        }  // f: 47-139
        repeat {  // f: 47-139
            step()  // f: 47-139
        } while running()  // f: 47-139
        defer {  // f: 77-79
            close()  // f: 47-139
        }  // f: 47-139
        do {  // f: 80-86
            try work()  // f: 47-139
        } catch let error as MyError {  // f: 47-139
            report(error)  // f: 47-139
        } catch {  // f: 47-139
            report(error)  // f: 47-139
        }  // f: 47-139
        switch kind {  // f: 87-95
        case .a:  // f: 88-90
            work()  // f: 47-139

        case let .b(n) where n > 0:  // f: 91-92
            print(n)  // f: 47-139
        @unknown default:  // f: 93-95
            break  // f: 47-139
        }  // f: 47-139
        let sum = start +  // f: 96-97
            end.distance(to: other)  // f: 47-139
        let list = [  // f: 98-101
            1,  // f: 47-139
            2,  // f: 47-139
        ]  // f: 47-139
        let map: [String: Int] = [  // f: 102-104
            "a": 1,  // f: 47-139
        ]  // f: 47-139
        let pair = (  // f: 105-108
            1,  // f: 47-139
            2  // f: 47-139
        )  // f: 47-139
        let made = Container<Int>(  // f: 47-139
            1  // f: 47-139
        )  // f: 47-139
        let nested = Outer.Inner<Int>(  // f: 112-114
            1  // f: 47-139
        )  // f: 47-139
        send(text,  // f: 115-118
             raw) { reply in  // f: 116-118
            print(reply)  // f: 47-139
        }  // f: 47-139

        animate {  // f: 120-121
            fade()  // f: 47-139
        } completion: {  // f: 122-124
            done()  // f: 47-139
        }  // f: 47-139
        UIView  // f: 125-130
            .animate {  // f: 126-127
                fade()  // f: 47-139
            } completion: {  // f: 128-130
                done()  // f: 47-139
            }  // f: 47-139
        print("x \" { y")  // f: 47-139
        app.get("hello") { req in  // f: 132-134
            req.body  // f: 47-139
        }  // f: 47-139
        /* a /* nested */ comment { */  // f: 47-139
        return items[  // f: 136-138
            0  // f: 47-139
        ]  // f: 47-139
    }  // f: 47-139
}  // f: 23-140

final class Panel: UIView {  // f: 142-161
    init(frame: CGRect) {  // f: 143-147
        super.init(  // f: 144-146
            frame: frame  // f: 143-147
        )  // f: 143-147
    }  // f: 143-147

    func make(  // f: 142-161
        a: Int  // f: 142-161
    ) -> Int {  // f: 151-153
        a  // f: 151-153
    }  // f: 151-153

    func pair() -> (  // f: 155-157
        Int,  // f: 142-161
        Int  // f: 142-161
    ) {  // f: 158-160
        (1, 2)  // f: 158-160
    }  // f: 158-160
}  // f: 142-161

extension Box {  // f: 163-169
    subscript(  // f: 163-169
        index: Int  // f: 163-169
    ) -> T {  // f: 166-168
        items[index]  // f: 166-168
    }  // f: 166-168
}  // f: 163-169

struct Named {  // f: 171-177
    func `default`(  // f: 171-177
        a: Int  // f: 171-177
    ) -> Int {  // f: 174-176
        a  // f: 174-176
    }  // f: 174-176
}  // f: 171-177
