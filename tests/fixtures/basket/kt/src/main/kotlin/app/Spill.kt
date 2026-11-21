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

fun spill() {
    jug().slosh()
    //    ^ d: kt/src/main/kotlin/app/Spill.kt:4
    Rack().forEach { jug().slosh() }
    //                     ^ d: picker kt/src/main/kotlin/app/Spill.kt:4, kt/src/main/kotlin/app/Spill.kt:8
}
