package plumes

class List<T> {
    operator fun get(i: Int): Blotter = Blotter()
}

class Plume {
    fun scrawl() {
    }
}

class Blotter {
    fun scrawl() {
    }
}

// A project `List<T>` whose `get` hands out a `Blotter`: `plumes[0]` is no `Plume` (#1610).
fun write(plumes: List<Plume>) {
    plumes[0].scrawl()
//            ^ d: !jump
}
