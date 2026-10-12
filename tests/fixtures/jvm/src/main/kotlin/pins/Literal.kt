package pins

fun String.quiet() = this

fun Int.quiet() = this

fun hush() {
    "hi".quiet()
    //   ^ d: src/main/kotlin/pins/Literal.kt:3
}
