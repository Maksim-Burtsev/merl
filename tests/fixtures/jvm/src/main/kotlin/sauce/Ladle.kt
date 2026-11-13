package sauce

// Pour declares a property and a local named Saucer. In Kotlin a receiver lambda such as
// `with(pour) { … }` sees that property by its bare name, which no line of the project shows,
// so the class Saucer stays Unknown here, as in Java it does not (#1514).
class Ladle : Saucer() {
    fun pour(): Int = super.splash()
    //                      ^ d: picker src/main/kotlin/sauce/Ramekin.kt:4, src/main/kotlin/sauce/Saucer.kt:4; want src/main/kotlin/sauce/Saucer.kt:4 (#1514)

    fun made(): Int = Saucer.make().splash()
    //                              ^ d: picker src/main/kotlin/sauce/Ramekin.kt:4, src/main/kotlin/sauce/Saucer.kt:4; want src/main/kotlin/sauce/Saucer.kt:4 (#1514)

    fun sift(v: Saucer): Int = v.splash()
    //                           ^ d: picker src/main/kotlin/sauce/Ramekin.kt:4, src/main/kotlin/sauce/Saucer.kt:4; want src/main/kotlin/sauce/Saucer.kt:4 (#1514)
}
