package sauce

// Pour declares a property and a local named Saucer. Nothing outside Pour reaches that property
// by its bare name (no subclass, extension or receiver lambda of Pour), so here Saucer is the
// class (#1514).
class Ladle : Saucer() {
    fun pour(): Int = super.splash()
    //                      ^ d: src/main/kotlin/sauce/Saucer.kt:4

    fun made(): Int = Saucer.make().splash()
    //                              ^ d: src/main/kotlin/sauce/Saucer.kt:4

    fun sift(v: Saucer): Int = v.splash()
    //                           ^ d: src/main/kotlin/sauce/Saucer.kt:4
}
