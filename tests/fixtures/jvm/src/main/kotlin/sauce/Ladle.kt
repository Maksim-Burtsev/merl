package sauce

// Pour declares a property and a local named Saucer: they stay its own and leave the class
// Saucer whole in this file (#1514).
class Ladle : Saucer() {
    fun pour(): Int = super.splash()
    //                      ^ d: src/main/kotlin/sauce/Saucer.kt:4

    fun made(): Int = Saucer.make().splash()
    //                              ^ d: src/main/kotlin/sauce/Saucer.kt:4

    fun sift(v: Saucer): Int = v.splash()
    //                           ^ d: src/main/kotlin/sauce/Saucer.kt:4
}
