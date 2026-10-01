// The type of a receiver (#384): a parameter's or a property's annotation, a construction of a
// type the project declares, a call's `-> Type`, `self`, a chain, an optional and a loop over
// `[Type]`; a type from outside that only the project extends; and what stays by name.
struct Instant {
    let value: Double

    static func - (lhs: Instant, rhs: Instant) -> Double {
        lhs.value - rhs.value
        //  ^ d: Sources/Shop/Receivers.swift:5
        //  status: value → Instant.value (via lhs: Instant)
    }
}

struct Clock {
    let value: Int
}

final class FormEncoder {
    func encode(_ value: Int) -> String {
        ""
    }
}

final class JSONPrinter {
    func encode(_ value: Int) -> [UInt8] {
        []
    }
}

final class ParameterEncoder {
    let encoder: FormEncoder = FormEncoder()
    var printers: [JSONPrinter] = []
    var backup: JSONPrinter?

    func run() -> String {
        encoder.encode(1)
        //      ^ d: Sources/Shop/Receivers.swift:19
        //        status: encode → FormEncoder.encode (via encoder: FormEncoder)
    }

    func make() -> String {
        let printer = FormEncoder()
        return printer.encode(2)
        //             ^ d: Sources/Shop/Receivers.swift:19
        //                 status: encode → FormEncoder.encode (via printer: FormEncoder)
    }

    func loop() {
        for p in printers {
            _ = p.encode(3)
            //    ^ d: Sources/Shop/Receivers.swift:25
        }
        _ = backup?.encode(4)
        //          ^ d: Sources/Shop/Receivers.swift:25
        _ = self.encoder.encode(5)
        //               ^ d: Sources/Shop/Receivers.swift:19
        //                   status: via self.encoder: FormEncoder
    }
}

final class Relay {
    let parameters = ParameterEncoder()

    func printer() -> JSONPrinter {
        JSONPrinter()
    }
}

func makeEncoder() -> FormEncoder {
    FormEncoder()
}

func relayed(_ relay: Relay) {
    _ = relay.parameters.encoder.encode(6)
    //                           ^ d: Sources/Shop/Receivers.swift:19
    let made = relay.printer()
    _ = made.encode(7)
    //       ^ d: Sources/Shop/Receivers.swift:25
    //           status: via relay.printer() -> JSONPrinter
    let built = try? makeEncoder()
    _ = built?.encode(8)
    //         ^ d: Sources/Shop/Receivers.swift:19
    if let backup = relay.parameters.backup {
        _ = backup.encode(9)
        //         ^ d: Sources/Shop/Receivers.swift:25
    }
}

// A type from outside: the project's extensions of it, and nothing else.
extension URLRequest {
    func watermarked() -> Bool {
        true
    }
}

func outside(_ request: URLRequest) {
    _ = request.watermarked()
    //          ^ d: Sources/Shop/Receivers.swift:91
    _ = request.encode(1)
    //          ^ d: none
}

// What proves nothing stays by name: a protocol, `any`, `some`, a generic parameter, even one
// named like a type, a tuple, a closure, and a construction of what the project only extends.
protocol Encoding {
    func encode(_ value: Int) -> String
}

func refused<FormEncoder: Encoding>(
    _ generic: FormEncoder,
    _ proto: Encoding,
    _ existential: any Encoding,
    _ opaque: some Encoding,
    _ pair: (JSONPrinter, Int),
    _ make: () -> JSONPrinter
) {
    _ = generic.encode(1)
    //          ^ d: picker Sources/Shop/Receivers.swift:19, Sources/Shop/Receivers.swift:25, Sources/Shop/Receivers.swift:106
    _ = proto.encode(1)
    //        ^ d: picker Sources/Shop/Receivers.swift:19, Sources/Shop/Receivers.swift:25, Sources/Shop/Receivers.swift:106
    _ = existential.encode(1)
    //              ^ d: picker Sources/Shop/Receivers.swift:19, Sources/Shop/Receivers.swift:25, Sources/Shop/Receivers.swift:106
    _ = opaque.encode(1)
    //         ^ d: picker Sources/Shop/Receivers.swift:19, Sources/Shop/Receivers.swift:25, Sources/Shop/Receivers.swift:106
    _ = pair.encode(1)
    //       ^ d: picker Sources/Shop/Receivers.swift:19, Sources/Shop/Receivers.swift:25, Sources/Shop/Receivers.swift:106
    _ = make.encode(1)
    //       ^ d: picker Sources/Shop/Receivers.swift:19, Sources/Shop/Receivers.swift:25, Sources/Shop/Receivers.swift:106
    let request = URLRequest()
    _ = request.watermarked()
    //          ^ d: Sources/Shop/Receivers.swift:91
    //                   status: by name
}
