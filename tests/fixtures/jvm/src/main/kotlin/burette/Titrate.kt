package burette

fun plain() {
    dispense().stopcock
    //         ^ d: src/main/kotlin/burette/Model.kt:4
}

fun plainPair() {
    val (a, b) = dispensePair()
    b.stopcock
    //^ d: src/main/kotlin/burette/Model.kt:8
}

fun withBody() = with(Dropper()) {
    dispense().stopcock
    //         ^ d: picker src/main/kotlin/burette/Model.kt:4, src/main/kotlin/burette/Model.kt:8
}

fun runBody() = Dropper().run {
    val c = dispense()
    c.stopcock
    //^ d: picker src/main/kotlin/burette/Model.kt:4, src/main/kotlin/burette/Model.kt:8
}

fun applyBody(): Dropper = Dropper().apply {
    dispense().stopcock
    //         ^ d: picker src/main/kotlin/burette/Model.kt:4, src/main/kotlin/burette/Model.kt:8
}

fun withPair() = with(Dropper()) {
    val (a, b) = dispensePair()
    b.stopcock
    //^ d: picker src/main/kotlin/burette/Model.kt:4, src/main/kotlin/burette/Model.kt:8
}

fun dsl() = dropping {
    dispense().stopcock
    //         ^ d: picker src/main/kotlin/burette/Model.kt:4, src/main/kotlin/burette/Model.kt:8
}

fun wrapped(
    n: Int = 1,
) = with(Dropper()) {
    dispense().stopcock
    //         ^ d: picker src/main/kotlin/burette/Model.kt:4, src/main/kotlin/burette/Model.kt:8
}

fun nested() {
    if (true) {
        dropping {
            dispense().stopcock
            //         ^ d: picker src/main/kotlin/burette/Model.kt:4, src/main/kotlin/burette/Model.kt:8
        }
    }
}

fun expression(): Int =
    dispense().stopcock
//             ^ d: src/main/kotlin/burette/Model.kt:4
