struct Review {
    var fault: Error?
    var job: Int?
}

struct Tally {
    let attempt: Int
    let crate: Int
    let tries: Int
}

func elsewhere() {
    let fault = 1
    let job = 2
}
