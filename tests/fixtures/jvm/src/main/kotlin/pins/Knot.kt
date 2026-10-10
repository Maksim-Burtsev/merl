package pins

class Knot(val up: Knot) {
    fun tie() = 1
}

class Bow {
    fun tie() = 2
}

fun climb(k: Knot) {
    k.up.up.up.up.up.tie()
    //               ^ d: src/main/kotlin/pins/Knot.kt:4
    k.up.up.up.up.up.up.tie()
    //                  ^ d: picker src/main/kotlin/pins/Knot.kt:4, src/main/kotlin/pins/Knot.kt:8
}
