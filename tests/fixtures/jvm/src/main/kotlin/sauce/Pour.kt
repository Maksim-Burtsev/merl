package sauce

// A property, a local and a parameter named like the class Saucer are values of type Ramekin
// (#1515).
class Pour {
    val Saucer = Ramekin()

    fun property(): Int = Saucer.splash()
    //                           ^ d: src/main/kotlin/sauce/Ramekin.kt:4

    fun local(): Int {
        val Saucer = Ramekin()
        return Saucer.splash()
        //            ^ d: src/main/kotlin/sauce/Ramekin.kt:4
    }

    fun param(Saucer: Ramekin): Int = Saucer.splash()
    //                                       ^ d: src/main/kotlin/sauce/Ramekin.kt:4
}
