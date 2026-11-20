package tankards

class Tankard {
    fun froth() {
    }
}

class Stein {
    fun froth() {
    }
}

fun Tankard(size: Int): Stein = Stein()

// `Tankard(1)` calls the function named like the class, so neither side of `to` is typed (#1610).
fun pourPair() {
    val (a, b) = Tankard(1) to Stein()
    a.froth()
//    ^ d: picker src/main/kotlin/tankards/Tankards.kt:4, src/main/kotlin/tankards/Tankards.kt:9
}
