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

    private object Keys : Tagged {
        override val label: String = "k"
    }

    // A nested object is a type, not a value of its supertype (#1515).
    fun keys(): String = Keys.label
    //                        ^ d: src/main/kotlin/sauce/Pour.kt:21

    // A companion object and a nested class do not see the property Saucer of Pour (#1515).
    companion object {
        fun made(): Int = Saucer.make().splash()
        //                              ^ d: src/main/kotlin/sauce/Saucer.kt:4
    }

    class Nested {
        fun made(): Int = Saucer.make().splash()
        //                              ^ d: src/main/kotlin/sauce/Saucer.kt:4
    }

    // A library lambda may take a Pour as its receiver, and an inner class sees the property
    // Saucer: both keep the picker (#1515).
    class Lidded {
        fun made(): Int = configure { Saucer.make().splash() }
        //                                          ^ d: picker src/main/kotlin/sauce/Ramekin.kt:4, src/main/kotlin/sauce/Saucer.kt:4
    }

    inner class Inner {
        fun made(): Int = Saucer.make().splash()
        //                              ^ d: picker src/main/kotlin/sauce/Ramekin.kt:4, src/main/kotlin/sauce/Saucer.kt:4
    }
}
