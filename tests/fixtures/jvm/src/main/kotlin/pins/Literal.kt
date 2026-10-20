package pins

fun String.quiet() = this

fun Int.quiet() = this

fun Long.quiet() = this

fun hush() {
    "hi".quiet()
    //   ^ d: src/main/kotlin/pins/Literal.kt:3
    val small = 7
    small.quiet()
    //    ^ d: src/main/kotlin/pins/Literal.kt:5
    val big = 10000000000
    big.quiet()
    //  ^ d: src/main/kotlin/pins/Literal.kt:7
}
