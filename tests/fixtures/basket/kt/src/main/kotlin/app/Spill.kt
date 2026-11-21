package app

class Jug {
    fun slosh() {}
}

class Vat {
    fun slosh() {}
}

class Rack {
    fun jug(): Vat = Vat()
}

fun jug(): Jug = Jug()

fun Rack.forEach(block: Rack.() -> Unit) = block()

class Shelf {
    fun jug(): Vat = Vat()
}

fun <R : Comparable<R>> Shelf.sortedBy(block: Shelf.() -> R): Shelf = this

fun spill() {
    jug().slosh()
    //    ^ d: kt/src/main/kotlin/app/Spill.kt:4
    Rack().forEach { jug().slosh() }
    //                     ^ d: picker kt/src/main/kotlin/app/Spill.kt:4, kt/src/main/kotlin/app/Spill.kt:8
    Shelf().sortedBy {
        jug().slosh()
        //    ^ d: picker kt/src/main/kotlin/app/Spill.kt:4, kt/src/main/kotlin/app/Spill.kt:8
        1
    }
}
